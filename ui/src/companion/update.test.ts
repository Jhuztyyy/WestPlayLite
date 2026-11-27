import { describe, expect, it, vi } from 'vitest'
import { noteLines } from './update'

// The app's line isn't there in a test (bridge.ts reads the page's address as it loads).
vi.mock('./bridge', () => ({ call: vi.fn(), on: vi.fn() }))

describe('noteLines', () => {
  // WestPlay Lite has no update notes (companion/update.ts): there is nothing to read from a changelog.
  it('has nothing, whatever the notes', () => {
    expect(noteLines('- Song lyrics on Now Playing.')).toEqual([])
    expect(noteLines('')).toEqual([])
    expect(noteLines('First release.')).toEqual([])
  })
})
