import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { describe, expect, spyOn, test } from 'bun:test'
import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseMetadata, readMetadataFile } from '../scripts/perf_direct'

const LIMIT = 8 * 1024 * 1024
const parse = (text: string) => parseMetadata(Buffer.from(text))
function temporary(action: (directory: string) => void) {
  const directory = fs.realpathSync(fs.mkdtempSync(join(tmpdir(), 'performance-metadata-')))
  try { action(directory) }
  finally { fs.rmSync(directory, { recursive: true, force: true }) }
}

describe('bounded outer metadata; no native owner', () => {
  test.skipIf(!process.env.PRISOMA_PERFORMANCE_MANIFEST)('explicit selected runtime manifest admits exact bytes', () => {
    const path = process.env.PRISOMA_PERFORMANCE_MANIFEST!
    const expected = process.env.PRISOMA_PERFORMANCE_MANIFEST_SHA256
    assert(typeof expected === 'string' && /^[0-9a-f]{64}$/.test(expected))
    const raw = readMetadataFile(path)
    expect(createHash('sha256').update(raw).digest('hex')).toBe(expected)
    const value = parseMetadata(raw)
    expect(typeof value.source_identity).toBe('string')
    expect(Object.hasOwn(value, 'project')).toBe(true)
  })
  test('ordinary floats and exponents retain native JSON value semantics', () => {
    const raw = '{"plain":1.25,"exponent":1e-6,"integer_float":1.0,"negative_zero":-0,"text":"a\\n\\u0062"}'
    const actual = parse(raw)
    expect(actual).toEqual(JSON.parse(raw))
    expect(Object.is(actual.negative_zero, -0)).toBe(true)
    expect(parse('{"left":{"same":1},"right":{"same":2}}')).toEqual({
      left: { same: 1 }, right: { same: 2 }
    })
  })
  test('duplicate decoded keys, overflow, malformed UTF-8 and syntax reject', () => {
    for (const raw of [
      '{"x":1,"x":2}', '{"x":1,"\\u0078":2}', '{"a\\u0062":1,"ab":2}',
      '{"outer":{"x":1,"x":2}}', '{"x":1e400}', '{"x":NaN}', '{"x":Infinity}',
      '{"x":[1,]}', '{"x":1} {"x":2}', '\ufeff{"x":1}',
    ]) expect(() => parse(raw)).toThrow()
    expect(() => parseMetadata(Buffer.from([0x7b, 0x22, 0xff, 0x22, 0x3a, 0x30, 0x7d]))).toThrow()
  })
  test('container depth32 admits and depth33 rejects before native parsing', () => {
    expect(() => parse('['.repeat(32) + '0' + ']'.repeat(32))).not.toThrow()
    expect(() => parse('['.repeat(33) + '0' + ']'.repeat(33))).toThrow()
  })
  test('per-object keys and array item limits admit exactly20000', () => {
    const record = Object.fromEntries(Array.from({ length: 20000 }, (_, i) => ['k'+i, null]))
    expect(Object.keys(parse(JSON.stringify(record))).length).toBe(20000)
    expect(() => parse(JSON.stringify({ ...record, excess: null }))).toThrow()
    expect(parse(JSON.stringify(Array(20000).fill(null))).length).toBe(20000)
    expect(() => parse(JSON.stringify(Array(20001).fill(null)))).toThrow()
  })
  test('value-node limit counts containers and admits exactly262144', () => {
    const value: null[][] = []
    let remaining = 262144 - 1 // The outer array is one value node.
    while (remaining) {
      const count = Math.min(19999, remaining - 1)
      value.push(Array(count).fill(null))
      remaining -= count + 1 // Each inner array is also a value node.
    }
    expect(parse(JSON.stringify(value)).length).toBe(value.length)
    value.at(-1)!.push(null)
    expect(() => parse(JSON.stringify(value))).toThrow()
  })
  test('regular max-sized file admits and excess is rejected before reading', () => {
    temporary(directory => {
      const path = join(directory, 'maximum.json')
      fs.writeFileSync(path, Buffer.concat([Buffer.from('0'), Buffer.alloc(LIMIT - 1, 0x20)]))
      const raw = readMetadataFile(path)
      expect(raw.length).toBe(LIMIT)
      expect(parseMetadata(raw)).toBe(0)
      fs.appendFileSync(path, ' ')
      const read = spyOn(fs, 'readSync')
      try {
        expect(() => readMetadataFile(path)).toThrow()
        expect(read).not.toHaveBeenCalled()
      } finally { read.mockRestore() }
    })
  })
  test('symlinks, nonregular files and source growth reject', () => {
    temporary(directory => {
      const path = join(directory, 'selected.json')
      fs.writeFileSync(path, '{"value":1}')
      expect(parseMetadata(readMetadataFile(path))).toEqual({ value: 1 })
      const link = join(directory, 'alias.json')
      fs.symlinkSync(path, link)
      expect(() => readMetadataFile(link)).toThrow()
      expect(() => readMetadataFile(directory)).toThrow()
      const original = fs.readSync
      let changed = false
      const read = spyOn(fs, 'readSync').mockImplementation((...args: any[]) => {
        if (!changed) {
          changed = true
          fs.appendFileSync(path, ' ')
        }
        return (original as any)(...args)
      })
      try {
        expect(() => readMetadataFile(path)).toThrow()
        expect(changed).toBe(true)
      } finally { read.mockRestore() }
      assert.equal(fs.readFileSync(path, 'utf8'), '{"value":1} ')
    })
  })
})

function failureControl(primary: unknown, readFails: boolean, closeFails: boolean) {
  const root = fs.realpathSync(fs.mkdtempSync(join(tmpdir(), 'metadata-cleanup-')))
  const path = join(root, 'input.json')
  fs.writeFileSync(path, '{"value":1}')
  const cleanup = new Error('synthetic descriptor close failure')
  const originalClose = fs.closeSync
  let descriptor: number | undefined, caught: unknown, rejected = false
  const read = readFails ? spyOn(fs, 'readSync').mockImplementation(() => { throw primary }) : null
  const close = spyOn(fs, 'closeSync').mockImplementation(fd => {
    descriptor = fd
    originalClose(fd)
    if (closeFails) throw cleanup
  })
  try {
    try { readMetadataFile(path) }
    catch (error) { rejected = true; caught = error }
  } finally { read?.mockRestore(); close.mockRestore() }
  try {
    expect(rejected).toBe(true)
    expect(descriptor).toBeDefined()
    expect(() => fs.fstatSync(descriptor!)).toThrow()
    if (readFails && closeFails) {
      expect(caught).toBeInstanceOf(AggregateError)
      const failures = (caught as AggregateError).errors
      expect(failures.length).toBe(2)
      expect(failures[0]).toBe(primary)
      expect(failures[1]).toBe(cleanup)
    } else expect(caught).toBe(readFails ? primary : cleanup)
  } finally { fs.rmSync(root, { recursive: true, force: true }) }
}

test('original read failure survives successful descriptor close', () => {
  failureControl(new Error('original read failure'), true, false)
})
test('successful read cannot hide descriptor close failure', () => {
  failureControl(undefined, false, true)
})
test('read plus descriptor close preserve both original failures', () => {
  failureControl(new Error('original read failure'), true, true)
})
test('undefined original failure remains an explicit grouped member', () => {
  failureControl(undefined, true, true)
})

test('escaped complete and truncated strings stay within a separately owned child bound', async () => {
  const subject = fileURLToPath(new URL('../scripts/perf_direct.ts', import.meta.url))
  const source = `
    import assert from 'node:assert/strict'
    import { parseMetadata } from ${JSON.stringify(subject)}
    const n = (8 * 1024 * 1024 - 2) / 2
    const valid = Buffer.alloc(2 + 2 * n)
    valid[0] = valid[valid.length - 1] = 34
    for (let k = 0; k < n; k++) { valid[1 + 2 * k] = 92; valid[2 + 2 * k] = 34 }
    const value = parseMetadata(valid)
    assert(value.length === n && value[0] === '"' && value[n - 1] === '"')
    assert.throws(() => parseMetadata(valid.subarray(0, valid.length - 1)), /Unterminated metadata string/)
    valid[valid.length - 1] = 92
    assert.throws(() => parseMetadata(valid), /Unterminated metadata escape/)
    for (const value of ['escaped quote " and slash \\\\', '\\n\\t', { 'a"': 'b"', '\\\\': ['\\\\"', 1e-6] }]) {
      const raw = Buffer.from(JSON.stringify(value))
      assert.deepEqual(parseMetadata(raw), value)
    }
    console.log(JSON.stringify({ regular_bytes: valid.length, escaped_quotes: n, malformed_rejections: 2 }))
  `
  const child = Bun.spawn([process.execPath, '--eval', source], { stdout: 'pipe', stderr: 'pipe' })
  let timedOut = false
  const timer = setTimeout(() => { timedOut = true; child.kill('SIGKILL') }, 10000)
  let returncode: number
  try { returncode = await child.exited }
  finally { clearTimeout(timer) }
  const stdout = await new Response(child.stdout).text()
  const stderr = await new Response(child.stderr).text()
  expect(timedOut).toBe(false)
  expect(stderr).toBe('')
  expect(returncode).toBe(0)
  expect(JSON.parse(stdout)).toEqual({ regular_bytes: LIMIT, escaped_quotes: (LIMIT - 2) / 2, malformed_rejections: 2 })
}, 15000)
