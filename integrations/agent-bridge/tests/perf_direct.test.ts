import { describe, expect, test } from 'bun:test'
import { diagnostic, extractPayloads } from '../scripts/perf_direct'

function fixture(k: number) {
  const camera = { id: 'front', width: 1, height: 1, periodTicks: 2 }
  const thermal = { id: 'heat', width: 1, height: 1, periodTicks: 3 }
  const plan = { profile: 'crebain.cpu-force-ground-environment.v1', sourceIdentity: 'a'.repeat(64),
    scene: { rgbCameras: [camera], thermalCameras: [thermal], microphones: [{ id: 'mic' }] } }
  const pressure = Buffer.alloc((Math.floor(k * 16000 / 120) - Math.floor((k - 1) * 16000 / 120)) * 8)
  for (let i = 0; i < pressure.length; i += 8) pressure.writeDoubleLE(i / 8 - 60, i)
  const heat = Buffer.alloc(4); heat.writeFloatLE(2.5)
  const batch = { tick: k, environmentProfile: plan.profile, sourceIdentity: plan.sourceIdentity,
    time: { numerator: k, denominator: 120, unit: 'second' },
    graphics: { tick: k, rowOrigin: 'bottom-left',
      rgb: k % 2 ? [] : [{ cameraId: 'front', width: 1, height: 1,
        encoding: 'rgba8-srgb', bytesBase64: Buffer.from([1, 2, 3, 255]).toString('base64') }],
      thermal: k % 3 ? [] : [{ cameraId: 'heat', width: 1, height: 1,
        encoding: 'float32-le', unit: 'W/(m2 sr)', bytesBase64: heat.toString('base64') }] },
    pressure: { sampleStart: Math.floor((k - 1) * 16000 / 120), sampleEnd: Math.floor(k * 16000 / 120),
      sampleRateHz: 16000, unit: 'pascal',
      channels: [{ microphoneId: 'mic', encoding: 'float64-le', bytesBase64: pressure.toString('base64') }] } }
  return { plan, batch, pressure }
}

describe('independent standalone payload observation decoder', () => {
  test('original errors, nested cleanup errors, and repeated identities survive', () => {
    const original = new Error('original failure')
    const cleanup = new Error('cleanup failure', { cause: original })
    const result = diagnostic(new AggregateError([original, cleanup], 'both'))
    expect(result.nodes.map(row => row.message)).toEqual(['both', 'original failure', 'cleanup failure'])
    expect(result.edges.length).toBe(3)
    expect(result.truncated).toBe(false)
  })
  test('due roster preserves complete original bytes and signed pressure', () => {
    const { plan, batch, pressure } = fixture(6)
    const rows = extractPayloads(batch, plan, 6)
    expect(rows.map(row => row.sensor_id)).toEqual(['rgb:front', 'thermal:heat', 'pressure:mic'])
    expect(rows[0].bytes).toEqual(Buffer.from([1, 2, 3, 255]))
    expect(rows[1].bytes.readFloatLE()).toBe(2.5)
    expect(rows[2].bytes).toEqual(pressure)
  })
  test('bounded repeated aggregate members report omitted edges', () => {
    const original = new Error('original failure')
    for (const count of [256, 257]) {
      const result = diagnostic(new AggregateError(Array(count).fill(original), 'repeated'))
      expect(result.nodes.map(row => row.message)).toEqual(['repeated', 'original failure'])
      expect(result.edges.length).toBe(256)
      expect(result.edges.every(row => row.from === 0 && row.to === 1 && row.kind === 'member')).toBe(true)
      expect(result.truncated).toBe(count > 256)
    }
  })
  test('nested diagnostics reserve ancestor edges before descending', () => {
    const leaf = new Error('leaf')
    for (const members of [244, 245]) {
      let root: Error = new AggregateError(Array(members).fill(leaf), 'members')
      for (let i = 0; i < 12; i++) root = new Error('ancestor', { cause: root })
      const result = diagnostic(root)
      expect(result.nodes.length).toBe(14)
      expect(result.edges.length).toBe(256)
      expect(result.edges.filter(row => row.kind === 'cause').length).toBe(12)
      expect(result.nodes.at(-1).message).toBe('leaf')
      expect(result.truncated).toBe(members > 244)
    }
  })
  test('not-due cameras do not become empty payloads', () => {
    const { plan, batch } = fixture(1)
    expect(extractPayloads(batch, plan, 1).map(row => row.sensor_id)).toEqual(['pressure:mic'])
    batch.graphics.rgb.push({ cameraId: 'front', width: 1, height: 1,
      encoding: 'rgba8-srgb', bytesBase64: 'AAAAAA==' })
    expect(() => extractPayloads(batch, plan, 1)).toThrow()
  })
  test('missing due and extra pressure source reject', () => {
    const a = fixture(6); a.batch.graphics.rgb = []
    expect(() => extractPayloads(a.batch, a.plan, 6)).toThrow()
    const b = fixture(6); b.batch.pressure.channels.push(b.batch.pressure.channels[0])
    expect(() => extractPayloads(b.batch, b.plan, 6)).toThrow()
  })
  test('foreign source, clock, axes, camera, and microphone reject', () => {
    const mutations = [
      (b: any) => { b.sourceIdentity = 'b'.repeat(64) },
      (b: any) => { b.time.denominator = 1000 },
      (b: any) => { b.tick = 7 },
      (b: any) => { b.graphics.rowOrigin = 'top-left' },
      (b: any) => { b.graphics.rgb[0].cameraId = 'other' },
      (b: any) => { b.pressure.channels[0].microphoneId = 'other' },
      (b: any) => { b.pressure.sampleEnd++ },
    ]
    for (const mutate of mutations) {
      const { batch, plan } = fixture(6); mutate(batch)
      expect(() => extractPayloads(batch, plan, 6)).toThrow()
    }
  })
  test('noncanonical base64 and wrong extent reject', () => {
    const a = fixture(6); a.batch.graphics.rgb[0].bytesBase64 = 'AQID/w==\n'
    expect(() => extractPayloads(a.batch, a.plan, 6)).toThrow()
    const b = fixture(6); b.batch.graphics.rgb[0].bytesBase64 = 'AQID'
    expect(() => extractPayloads(b.batch, b.plan, 6)).toThrow()
  })
  test('non-finite scientific arrays and wrong thermal unit reject', () => {
    const a = fixture(6); const heat = Buffer.alloc(4); heat.writeFloatLE(Infinity)
    a.batch.graphics.thermal[0].bytesBase64 = heat.toString('base64')
    expect(() => extractPayloads(a.batch, a.plan, 6)).toThrow()
    const b = fixture(6); b.pressure.writeDoubleLE(NaN)
    b.batch.pressure.channels[0].bytesBase64 = b.pressure.toString('base64')
    expect(() => extractPayloads(b.batch, b.plan, 6)).toThrow()
    const c = fixture(6); c.batch.graphics.thermal[0].unit = 'kelvin'
    expect(() => extractPayloads(c.batch, c.plan, 6)).toThrow()
  })
})
