import { describe, expect, it } from 'vitest'
import { formatText } from './format'

describe('formatText', () => {
  it('formats JSON with two spaces', async () => {
    expect(await formatText('{"a":1,"b":[1,2]}', 'json')).toBe(
      '{ "a": 1, "b": [1, 2] }\n',
    )
  })

  it('keeps YAML comments', async () => {
    const out = await formatText('a:   1 # keep me\nb:\n  - x\n', 'yaml')
    expect(out).toContain('# keep me')
    expect(out).toContain('a: 1')
  })

  it('rejects broken JSON', async () => {
    await expect(formatText('{ nope', 'json')).rejects.toThrow()
  })
})
