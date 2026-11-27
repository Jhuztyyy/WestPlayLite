import { describe, expect, it } from 'vitest'
import { isReleasedApp, RELEASED_APP_IDS } from './apps'

describe('the apps this release has', () => {
  it('leaves Podcasts for a later release, keeping Home’s order', () => {
    expect(RELEASED_APP_IDS).toEqual([
      'music',
      'spotify',
      'youtube',
      'youtubeMusic',
      'player',
      'safari',
      'settings',
    ])
  })

  it('opens none of the later ones from a saved screen or recent', () => {
    expect(isReleasedApp('podcasts')).toBe(false)
    expect(isReleasedApp('phone')).toBe(false)
    expect(isReleasedApp('music')).toBe(true)
    expect(isReleasedApp('nonsense')).toBe(false)
  })
})
