import { describe, expect, it, vi } from 'vitest'

const video = (id: string) => ({
  video: id,
  title: '',
  channel: '',
  avatar: null,
  thumbnail: null,
  duration: '',
  views: '',
  age: '',
  live: false,
})

describe('a video a phone cast', () => {
  it('is a cast one while it plays, and no longer once the player plays another', async () => {
    // The page around it (outside WestPlay): no host to talk to, no storage kept.
    vi.stubGlobal('window', {})
    vi.stubGlobal('location', { search: '' })
    const items = new Map<string, string>()
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => items.get(key) ?? null,
      setItem: (key: string, value: string) => void items.set(key, value),
      removeItem: (key: string) => void items.delete(key),
    })
    const { isCast, play, playCast, useYouTube } = await import('./youtube')
    expect(isCast()).toBe(false)
    playCast(video('AAAAAAAAAAA'))
    expect(useYouTube.getState().current?.video).toBe('AAAAAAAAAAA')
    expect(isCast()).toBe(true)
    // The phone's next video comes as a cast of its own.
    playCast(video('BBBBBBBBBBB'))
    expect(isCast()).toBe(true)
    // The player's own pick (a tap on the truck's screen): autoplay is its own again.
    play(video('CCCCCCCCCCC'))
    expect(isCast()).toBe(false)
  })
})

describe('a cast video about to end', () => {
  it('is held under a second and a half before its end, only while it plays', async () => {
    vi.stubGlobal('window', {})
    vi.stubGlobal('location', { search: '' })
    const { castEnding } = await import('./youtube')
    const state = (over: Record<string, unknown>) =>
      ({ video: 'AAAAAAAAAAA', state: 1, live: false, duration: 200, ...over }) as never
    expect(castEnding(state({}), 'AAAAAAAAAAA', 100)).toBe(false)
    expect(castEnding(state({}), 'AAAAAAAAAAA', 198.4)).toBe(false)
    expect(castEnding(state({}), 'AAAAAAAAAAA', 198.6)).toBe(true)
    // Not playing, live, a short clip, another video's state, or none: not held.
    expect(castEnding(state({ state: 2 }), 'AAAAAAAAAAA', 199)).toBe(false)
    expect(castEnding(state({ live: true }), 'AAAAAAAAAAA', 199)).toBe(false)
    expect(castEnding(state({ duration: 3 }), 'AAAAAAAAAAA', 2.8)).toBe(false)
    expect(castEnding(state({ video: 'BBBBBBBBBBB' }), 'AAAAAAAAAAA', 199)).toBe(false)
    expect(castEnding(null, 'AAAAAAAAAAA', 199)).toBe(false)
  })
})
