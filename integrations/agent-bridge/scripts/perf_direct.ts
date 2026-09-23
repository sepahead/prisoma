import assert from 'node:assert/strict'
import * as fs from 'node:fs'
import { createHash } from 'node:crypto'
import { constants, openSync, writeSync, fsyncSync, closeSync, mkdirSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const MAX_SPANS = 32768
const MAX_INPUT = 8 * 1024 * 1024
const MAX_OUTPUT = 48 * 1024 * 1024
const sha = (bytes: string | Uint8Array) => createHash('sha256').update(bytes).digest('hex')
const base = process.hrtime.bigint()
let last = 0
function now(): number {
  const value = Number(process.hrtime.bigint() - base)
  assert(Number.isSafeInteger(value) && value >= last)
  last = value
  return value
}
const MAX_METADATA_NODES = 262144
const MAX_METADATA_DEPTH = 32
const MAX_METADATA_MEMBERS = 20000

/** Outer file admission only. The measured observation loop keeps its native decoder. */
export function readMetadataFile(path: string): Buffer {
  assert(typeof path === 'string' && resolve(path) === path && fs.realpathSync(path) === path)
  const fd = fs.openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK)
  let failed = false, primary: unknown
  try {
    const before = fs.fstatSync(fd, { bigint: true })
    assert(before.isFile() && before.size > 0n && before.size <= BigInt(MAX_INPUT))
    const buffer = Buffer.alloc(Number(before.size) + 1)
    let used = 0
    while (used < buffer.length) {
      const count = fs.readSync(fd, buffer, used, buffer.length - used, null)
      if (count === 0) break
      used += count
    }
    assert(BigInt(used) === before.size)
    const descriptor = fs.fstatSync(fd, { bigint: true })
    const lexical = fs.lstatSync(path, { bigint: true })
    for (const key of ['dev', 'ino', 'mode', 'uid', 'gid', 'size', 'mtimeNs', 'ctimeNs'] as const) {
      assert(before[key] === descriptor[key] && before[key] === lexical[key])
    }
    return buffer.subarray(0, used)
  } catch (error) { failed = true; primary = error; throw error }
  finally {
    try { fs.closeSync(fd) }
    catch (cleanup) {
      if (failed) throw new AggregateError([primary, cleanup], 'Metadata read and descriptor cleanup failed')
      throw cleanup
    }
  }
}

/** Admit outer JSON structure and duplicates, retaining JSON.parse number semantics. */
export function parseMetadata(raw: Uint8Array): any {
  assert(raw.byteLength > 0 && raw.byteLength <= MAX_INPUT)
  const text = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(raw)
  type Container = { kind: 'object'; keys: Set<string>; expectKey: boolean }
    | { kind: 'array'; items: number }
  const stack: Container[] = []
  let nodes = 0
  function admitValue() {
    assert(++nodes <= MAX_METADATA_NODES)
    const parent = stack.at(-1)
    if (parent?.kind === 'array') assert(++parent.items <= MAX_METADATA_MEMBERS)
  }
  // The cursor never retreats. Every code unit belongs to one token or whitespace.
  // JSON.parse still owns complete syntax and value semantics after admission.
  const whitespace = (value: string) => value === ' ' || value === '\t' || value === '\r' || value === '\n'
  const delimiter = (value: string) => whitespace(value) || '{}[],:"'.includes(value)
  for (let cursor = 0; cursor < text.length;) {
    if (whitespace(text[cursor])) { cursor++; continue }
    const start = cursor++
    let token = text[start]
    if (token === '"') {
      let closed = false
      while (cursor < text.length) {
        const current = text[cursor++]
        if (current === '\\') {
          assert(cursor < text.length, 'Unterminated metadata escape')
          cursor++
        } else if (current === '"') { closed = true; break }
      }
      assert(closed, 'Unterminated metadata string')
      token = text.slice(start, cursor)
    } else if (!'{}[],:'.includes(token)) {
      while (cursor < text.length && !delimiter(text[cursor])) cursor++
      token = text.slice(start, cursor)
    }
    if (token === '{' || token === '[') {
      admitValue()
      stack.push(token === '{' ? { kind: 'object', keys: new Set(), expectKey: true }
        : { kind: 'array', items: 0 })
      assert(stack.length <= MAX_METADATA_DEPTH)
    } else if (token === '}' || token === ']') {
      const parent = stack.pop()
      assert(parent?.kind === (token === '}' ? 'object' : 'array'))
    } else if (token === ',') {
      const parent = stack.at(-1)
      if (parent?.kind === 'object') parent.expectKey = true
    } else if (token !== ':') {
      const parent = stack.at(-1)
      if (token[0] === '"' && parent?.kind === 'object' && parent.expectKey) {
        const key = JSON.parse(token)
        assert(!parent.keys.has(key), 'Duplicate metadata key')
        parent.keys.add(key)
        assert(parent.keys.size <= MAX_METADATA_MEMBERS)
        parent.expectKey = false
      } else admitValue()
    }
  }
  assert(stack.length === 0)
  const value = JSON.parse(text)
  const pending = [value]
  while (pending.length) {
    const current = pending.pop()
    if (typeof current === 'number') assert(Number.isFinite(current), 'Nonfinite metadata number')
    if (current && typeof current === 'object') {
      for (const child of Object.values(current)) pending.push(child)
    }
  }
  // Every object key owns a value node; a separate equal global-key cap adds no bound.
  return value
}

function selected(row: any): Buffer {
  const raw = readMetadataFile(row.path)
  assert(raw.length === row.bytes && sha(raw) === row.sha256)
  return raw
}
function canonical(value: any): string {
  const sort = (v: any): any => Array.isArray(v) ? v.map(sort)
    : v !== null && typeof v === 'object'
      ? Object.fromEntries(Object.keys(v).sort().map(key => [key, sort(v[key])]))
      : v
  return JSON.stringify(sort(value)).replace(/[\u007f-\uffff]/g,
    value => '\\u' + value.charCodeAt(0).toString(16).padStart(4, '0'))
}

type Span = { id: number; parent: number | null; label: string; tick: number;
  start_ns: number; end_ns: number | null; outcome: string }
const spans: Span[] = []
const stack: number[] = []
let tick = 0
let spanOverflow = false
function validateSpans() {
  assert(stack.length === 0 && !spanOverflow)
  for (const row of spans) {
    assert(row.end_ns !== null && row.end_ns >= row.start_ns)
    assert(['returned', 'exception'].includes(row.outcome))
    if (row.parent !== null) {
      assert(row.parent >= 0 && row.parent < row.id)
      const parent = spans[row.parent]
      assert(parent.start_ns <= row.start_ns && parent.end_ns !== null && parent.end_ns >= row.end_ns)
    }
  }
}

export function diagnostic(root: unknown) {
  const ids = new Map<unknown, number>()
  const nodes: any[] = [], edges: any[] = []
  let truncated = false
  let admittedEdges = 0
  function visit(value: unknown): number | null {
    if (ids.has(value)) return ids.get(value)!
    if (nodes.length >= 128) { truncated = true; return null }
    const id = nodes.length
    ids.set(value, id)
    nodes.push({ id, type: value instanceof Error ? value.name : typeof value,
      message: value instanceof Error ? value.message.slice(0, 2048) : 'Non-Error failure' })
    const children: Array<[string, unknown]> = []
    if (value instanceof Error && value.cause !== undefined) children.push(['cause', value.cause])
    if (value instanceof AggregateError) {
      if (value.errors.length > 256) truncated = true
      for (const member of value.errors.slice(0, 256)) children.push(['member', member])
    }
    for (const [kind, child] of children) {
      if (admittedEdges >= 256) { truncated = true; break }
      // Reserve before recursion so descendants cannot spend an ancestor's edge.
      admittedEdges++
      const to = visit(child)
      if (to !== null) edges.push({ from: id, to, kind })
    }
    return id
  }
  visit(root)
  return { nodes, edges, truncated }
}
async function span<T>(label: string, action: () => Promise<T> | T): Promise<T> {
  if (spans.length >= MAX_SPANS) { spanOverflow = true; throw new Error('Span capacity') }
  const id = spans.length
  const row: Span = { id, parent: stack.at(-1) ?? null, label, tick,
    start_ns: now(), end_ns: null, outcome: 'pending' }
  spans.push(row); stack.push(id)
  let primary: unknown
  try {
    const result = await action()
    row.outcome = 'returned'
    return result
  } catch (error) { primary = error; row.outcome = 'exception'; throw error }
  finally {
    try { row.end_ns = now(); assert(stack.pop() === id) }
    catch (error) {
      if (primary !== undefined) throw new AggregateError([primary, error], 'Operation and timing cleanup failed')
      throw error
    }
  }
}
let totalOutput = 0
function writeAll(fd: number, raw: Buffer) {
  assert(totalOutput + raw.length <= MAX_OUTPUT)
  let offset = 0
  while (offset < raw.length) {
    const n = writeSync(fd, raw, offset, raw.length - offset)
    assert(n > 0)
    offset += n
  }
  totalOutput += raw.length
}
let output = ''
function save(name: string, raw: Buffer) {
  assert(!name.includes('/') && name.length > 0)
  const fd = openSync(join(output, name), constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW, 0o600)
  try { writeAll(fd, raw); fsyncSync(fd) } finally { closeSync(fd) }
  return { path: name, bytes: raw.length, sha256: sha(raw) }
}
function saveJson(name: string, value: unknown) {
  return save(name, Buffer.from(canonical(value) + '\n'))
}
function payload(text: unknown, count: number, kind: string): Buffer {
  assert(typeof text === 'string' && Number.isSafeInteger(count) && count > 0)
  assert(text.length === 4 * Math.ceil(count / 3))
  const raw = Buffer.from(text, 'base64')
  assert(raw.length === count && raw.toString('base64') === text)
  if (kind !== 'rgba8') {
    const size = kind === 'pressure' ? 8 : 4
    for (let i = 0; i < raw.length; i += size)
      assert(Number.isFinite(size === 8 ? raw.readDoubleLE(i) : raw.readFloatLE(i)))
  }
  return raw
}

export function extractPayloads(batch: any, plan: any, k: number) {
  assert(batch.tick === k && batch.environmentProfile === plan.profile)
  assert(batch.time.numerator === k && batch.time.denominator === 120 && batch.time.unit === 'second')
  assert(batch.sourceIdentity === plan.sourceIdentity)
  assert(batch.graphics !== null && batch.graphics.tick === k && batch.graphics.rowOrigin === 'bottom-left')
  const result: Array<{ sensor_id: string; bytes: Buffer }> = []
  for (const [modality, roster, kind] of [
    ['rgb', plan.scene.rgbCameras, 'rgba8'], ['thermal', plan.scene.thermalCameras, 'radiance']
  ] as const) {
    const due = roster.filter((camera: any) => k % camera.periodTicks === 0)
    const frames = batch.graphics[modality]
    assert(Array.isArray(frames) && frames.length === due.length)
    frames.forEach((frame: any, i: number) => {
      const camera = due[i]
      assert(frame.cameraId === camera.id && frame.width === camera.width && frame.height === camera.height)
      assert(frame.encoding === (kind === 'rgba8' ? 'rgba8-srgb' : 'float32-le'))
      if (kind === 'radiance') assert(frame.unit === 'W/(m2 sr)')
      result.push({ sensor_id: modality + ':' + camera.id,
        bytes: payload(frame.bytesBase64, camera.width * camera.height * 4, kind) })
    })
  }
  const pressure = batch.pressure
  const start = Math.floor((k - 1) * 16000 / 120), end = Math.floor(k * 16000 / 120)
  assert(pressure.sampleStart === start && pressure.sampleEnd === end)
  assert(pressure.sampleRateHz === 16000 && pressure.unit === 'pascal')
  assert(pressure.channels.length === plan.scene.microphones.length)
  pressure.channels.forEach((channel: any, i: number) => {
    assert(channel.microphoneId === plan.scene.microphones[i].id && channel.encoding === 'float64-le')
    result.push({ sensor_id: 'pressure:' + channel.microphoneId,
      bytes: payload(channel.bytesBase64, (end - start) * 8, 'pressure') })
  })
  return result
}

async function run(freezePath: string, caseId: string) {
  const freezeRaw = readMetadataFile(freezePath)
  const freeze = parseMetadata(freezeRaw)
  assert(freeze.schema === 'local.m1-performance-freeze.v1')
  const cases = freeze.cases.filter((x: any) => x.case_id === caseId)
  assert(cases.length === 1 && cases[0].route === 'direct')
  const selectedCase = cases[0]
  for (const row of Object.values(freeze.tools)) selected(row)
  const self = selected(freeze.tools.direct_worker)
  assert(sha(self) === sha(readMetadataFile(fileURLToPath(import.meta.url))))
  const workload = parseMetadata(selected(freeze.workload))
  // Include this route's manifest/source join and native imports in preparation.
  // Full installed-tree verification belongs to the selected Python launcher.
  const prepareStart = now()
  const manifestRaw = readMetadataFile(join(freeze.runtime.prefix, 'runtime.json'))
  assert(sha(manifestRaw) === freeze.runtime.manifest_sha256)
  const manifest = parseMetadata(manifestRaw)
  assert(manifest.source_identity === freeze.runtime.source_identity)
  const project = join(freeze.runtime.prefix, 'project')
  // These are the installed standalone CREBAIN APIs, not another project's mutable owner.
  const { EnvironmentOwner, observationEnvelopeBytes } =
    await import(pathToFileURL(join(project, 'src/environment/EnvironmentOwner.ts')).href)
  const { EnvironmentState } =
    await import(pathToFileURL(join(project, 'src/environment/EnvironmentState.ts')).href)
  const { OwnedGraphicsProcess } =
    await import(pathToFileURL(join(project, 'scripts/lib/owned-graphics-process.mjs')).href)
  const plan = { ...workload.specification, runId: 'ncp-' + selectedCase.run_id,
    sourceIdentity: freeze.runtime.source_identity }
  const horizon = workload.planned_ticks
  assert(Number.isSafeInteger(horizon) && horizon >= 1 && horizon <= 1022)
  const actions = new Map(workload.targets.map((action: any) => [action.tick, action]))
  assert(actions.size === workload.targets.length && actions.has(1))
  const maximum = [...plan.scene.rgbCameras, ...plan.scene.thermalCameras]
    .reduce((sum: number, camera: any) => sum + camera.width * camera.height * 4, 0)
    + plan.scene.microphones.length * 134 * 8
  output = join(freeze.output_root, caseId)
  mkdirSync(output, { mode: 0o700 })
  saveJson('case.json', { freeze_sha256: sha(freezeRaw), case: selectedCase,
    clock_domain: 'process-local-hrtime-bigint', clock_id: selectedCase.clock_id,
    runtime: freeze.runtime, owner_api: 'EnvironmentOwner' })
  const detailed = selectedCase.instrumentation === 'detailed'
  const originalControlled = EnvironmentState.prototype.advanceControlled
  if (detailed)
    EnvironmentState.prototype.advanceControlled = async function(...args: any[]) {
      return span('native.cpu_pressure_thermal_inclusive', () => originalControlled.apply(this, args))
    }
  let owner: any, graphics: any, primary: unknown = null
  let origin: number | null = null, prepareEnd: number | null = null
  let finishStart: number | null = null, finishEnd: number | null = null, retired: number | null = null
  let count = 0, observedTicks = 0, releasedTicks = 0
  let cleanupConfirmed = false
  let incompleteTick: any = null
  const rows: any[] = [], observations: string[] = []
  const stepFd = openSync(join(output, 'steps.jsonl'), constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL, 0o600)
  let stopped = false
  const stop = () => { stopped = true }
  process.on('SIGINT', stop)
  process.on('SIGTERM', stop)
  try {
    owner = await span('owner.prepare_inclusive', () => EnvironmentOwner.prepare(plan,
      async (json: string) => {
        graphics = await OwnedGraphicsProcess.prepare(json, { timeoutMs: 30000, nodeExecutable: freeze.node })
        if (!detailed) return graphics
        return {
          diagnostics: () => graphics.diagnostics(),
          captureJson: (input: string) => span('native.graphics_capture_inclusive', () => graphics.captureJson(input)),
          retire: () => span('native.graphics_retire', () => graphics.retire())
        }
      }, maximum, observationEnvelopeBytes(plan.profile, maximum)))
    prepareEnd = now(); origin = prepareEnd
    const sessionDeadline = prepareStart + freeze.limits.session_seconds * 1e9
    for (tick = 1; tick <= horizon; tick++) {
      assert(!stopped && now() < sessionDeadline, 'Direct session deadline or interruption')
      const offer = origin + Math.ceil((tick - 1) * 1e9 / 120)
      const deadline = origin * 120 + tick * 1e9
      while (now() < offer)
        await new Promise(resolve => setTimeout(resolve, Math.max(0, (offer - now()) / 1e6)))
      const start = now()
      incompleteTick = { tick, offer_ns: offer, deadline_numerator_ns_x120: deadline, start_ns: start }
      let readings: Array<{ sensor_id: string; bytes: Buffer }> = []
      let observationNs = 0, originalJson = ''
      await span('route.advance_inclusive', async () => {
        const action = actions.get(tick)
        if (action) await span('native.schedule', () => owner.schedule(action))
        const handle = await span('native.advance', () => owner.advance())
        readings = await span('native.read_and_decode', () => {
          originalJson = owner.readObservation(handle)
          assert(handle.ownerId === owner.ownerId && handle.tick === tick && handle.sha256 === sha(originalJson))
          const batch = JSON.parse(originalJson)
          assert(batch.ownerId === owner.ownerId)
          return extractPayloads(batch, plan, tick)
        })
        observationNs = now(); observedTicks = tick
        await span('route.release', () => owner.releaseObservation(handle))
        releasedTicks = tick
      })
      const routeComplete = now()
      incompleteTick.observation_ns = observationNs
      incompleteTick.route_complete_ns = routeComplete
      await span('benchmark.hash_and_durable_export', () => {
        const payloads = readings.map(reading => {
          const identity = save('payload-' + String(count++).padStart(5, '0') + '.bin', reading.bytes)
          return { ...identity, sensor_id: reading.sensor_id }
        })
        writeAll(stepFd, Buffer.from(canonical({ tick, payloads }) + '\n'))
        fsyncSync(stepFd)
      })
      const exported = now()
      rows.push({ tick, clock_id: selectedCase.clock_id, offer_ns: offer,
        deadline_numerator_ns_x120: deadline, start_ns: start, observation_ns: observationNs,
        route_complete_ns: routeComplete, export_complete_ns: exported,
        route_missed: routeComplete * 120 > deadline, export_missed: exported * 120 > deadline })
      incompleteTick = null
      observations.push(originalJson)
    }
    finishStart = now()
    await span('route.finish_and_owner_retirement', () => owner.retire())
    finishEnd = now(); retired = finishEnd; cleanupConfirmed = true
    assert(owner.status().acceptedObservationTick === horizon && !stopped)
  } catch (error) { primary = error }
  finally {
    try { fsyncSync(stepFd) }
    catch (error) { primary = primary === null ? error : new AggregateError([primary, error], 'Operation and final export synchronization failed') }
    try { closeSync(stepFd) }
    catch (error) { primary = primary === null ? error : new AggregateError([primary, error], 'Operation and export close failed') }
    if (owner && !cleanupConfirmed) {
      try { await owner.retire(); cleanupConfirmed = true; retired = now() }
      catch (cleanup) { primary = primary === null ? cleanup : new AggregateError([primary, cleanup], 'Operation and cleanup failed') }
    }
    if (!owner && graphics) {
      // A returned graphics owner can be retired, but absent EnvironmentOwner
      // authority does not establish successful CPU preparation cleanup.
      try { await graphics.retire(); retired = now() }
      catch (cleanup) { primary = primary === null ? cleanup : new AggregateError([primary, cleanup], 'Preparation and cleanup failed') }
    }
    EnvironmentState.prototype.advanceControlled = originalControlled
    process.off('SIGINT', stop); process.off('SIGTERM', stop)
  }
  try { validateSpans() }
  catch (error) { primary = primary === null ? error : new AggregateError([primary, error], 'Operation and span validation failed') }
  // Original native JSON remains audit data and is written outside per-tick timing.
  try {
    observations.forEach((json, i) => save('native-batch-' + String(i + 1).padStart(3, '0') + '.json', Buffer.from(json)))
    saveJson('result.json', { schema: 'local.m1-performance-case.v1', case: selectedCase,
    freeze_sha256: sha(freezeRaw), status: primary === null ? 'complete' : 'failed',
    scope: 'coarse instrumented local timing; no real-time or release qualification',
    clock_id: selectedCase.clock_id, clock_domain: 'process-local-hrtime-bigint',
    planned_ticks: horizon, completed_ticks: rows.length,
    completed_ticks_meaning: 'complete route and benchmark-export measurements',
    last_observed_tick: observedTicks, last_released_tick: releasedTicks,
    uncompleted_measurement_ticks: Array.from({ length: horizon - rows.length }, (_, i) => rows.length + i + 1),
    incomplete_tick: incompleteTick, prepare_start_ns: prepareStart, prepare_end_ns: prepareEnd,
    preparation_scope: 'manifest/source join, installed native imports, output setup, and standalone owner admission; full InstalledRuntime verification is separately performed by campaign launcher',
    offered_origin_ns: origin, finish_start_ns: finishStart, finish_end_ns: finishEnd, retired_ns: retired,
    ticks: rows, spans, span_overflow: spanOverflow, wire: null, payload_count: count,
    cleanup_confirmed: cleanupConfirmed, owner_status: owner?.status() ?? null,
    graphics_diagnostics: graphics?.diagnostics() ?? null,
    failure: primary === null ? null : diagnostic(primary) })
  } catch (error) {
    if (primary !== null) throw new AggregateError([primary, error], 'Operation and terminal publication failed')
    throw error
  }
  if (primary !== null) throw primary
  console.log(JSON.stringify({ case: caseId, status: 'complete', ticks: rows.length,
    route_deadline_misses: rows.filter(row => row.route_missed).length,
    export_deadline_misses: rows.filter(row => row.export_missed).length }))
}

if (import.meta.main) {
  const [freezePath, caseId] = process.argv.slice(2)
  assert(freezePath && caseId && process.argv.length === 4)
  await run(freezePath, caseId)
}
