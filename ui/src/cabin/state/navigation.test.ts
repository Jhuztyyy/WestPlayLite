import { describe, expect, it } from 'vitest'
import {
  homeButtonTarget,
  INITIAL_NAVIGATION,
  navigate,
  type NavigationState,
  RECENTS_LIMIT,
  routeKey,
  sidebarApps,
} from './navigation'
import { APP_IDS } from './apps'

describe('navigate', () => {
  it('opens an app and moves it to the front of the recents', () => {
    const state = navigate(INITIAL_NAVIGATION, { type: 'openApp', app: 'youtube' })
    expect(state.route).toEqual({ kind: 'app', app: 'youtube' })
    expect(state.recents).toEqual(['youtube', 'music', 'spotify', 'youtubeMusic'])
  })

  it('adds a new app to the recents, keeping every app used', () => {
    const state = navigate(INITIAL_NAVIGATION, { type: 'openApp', app: 'settings' })
    expect(state.recents).toEqual(['settings', 'music', 'youtube', 'spotify', 'youtubeMusic'])
    expect(RECENTS_LIMIT).toBe(APP_IDS.length)
  })

  it('takes the Home button from an app back to where the app was opened from', () => {
    const fromHome = navigate(
      { ...INITIAL_NAVIGATION, route: { kind: 'home' }, base: 'home' },
      {
        type: 'openApp',
        app: 'music',
      },
    )
    expect(homeButtonTarget(fromHome)).toBe('home')
    expect(navigate(fromHome, { type: 'pressHome' }).route).toEqual({ kind: 'home' })

    const fromDashboard = navigate(INITIAL_NAVIGATION, { type: 'openApp', app: 'music' })
    expect(homeButtonTarget(fromDashboard)).toBe('dashboard')
    expect(navigate(fromDashboard, { type: 'pressHome' }).route).toEqual({ kind: 'dashboard' })
  })

  it('remembers the last of Home and Dashboard across apps', () => {
    const home = navigate(INITIAL_NAVIGATION, { type: 'pressHome' })
    const inApp = navigate(navigate(home, { type: 'openApp', app: 'youtube' }), { type: 'openApp', app: 'music' })
    expect(homeButtonTarget(inApp)).toBe('home')
  })

  it('toggles the Home button between Home and Dashboard', () => {
    const home: NavigationState = { ...INITIAL_NAVIGATION, route: { kind: 'home' }, base: 'home' }
    expect(homeButtonTarget(home)).toBe('dashboard')
    const dashboard = navigate(home, { type: 'pressHome' })
    expect(dashboard.route).toEqual({ kind: 'dashboard' })
    expect(dashboard.base).toBe('dashboard')
    expect(homeButtonTarget(dashboard)).toBe('home')
    expect(navigate(dashboard, { type: 'pressHome' }).route).toEqual({ kind: 'home' })
  })

  it('does not touch the recents when navigating between Home and Dashboard', () => {
    const state = navigate(INITIAL_NAVIGATION, { type: 'pressHome' })
    expect(state.recents).toBe(INITIAL_NAVIGATION.recents)
  })

  it('shows the Dashboard', () => {
    const inApp: NavigationState = { ...INITIAL_NAVIGATION, route: { kind: 'app', app: 'youtube' } }
    expect(navigate(inApp, { type: 'showDashboard' }).route).toEqual({ kind: 'dashboard' })
    // Home itself, whether from an app or from Home (where the Home button would go to the Dashboard).
    expect(navigate(inApp, { type: 'showHome' }).route).toEqual({ kind: 'home' })
    const home = navigate(inApp, { type: 'showHome' })
    expect(navigate(home, { type: 'showHome' })).toEqual(home)
  })
})

describe('sidebarApps', () => {
  it('shows one app of each kind, the last used of it, in a fixed order', () => {
    // The last used music app, then Settings or Safari (YouTube never takes a slot). No Maps, Phone or Messages.
    expect(sidebarApps(['youtube', 'spotify', 'music'], 2)).toEqual(['spotify', 'settings'])
    expect(sidebarApps(['youtubeMusic', 'safari', 'spotify'], 2)).toEqual(['youtubeMusic', 'safari'])
  })

  it("takes each kind's first app until one of it is used", () => {
    expect(sidebarApps([], 2)).toEqual(['music', 'settings'])
    expect(sidebarApps(['youtube'], 2)).toEqual(['music', 'settings'])
  })

  it('shows only as many as the screen has room for', () => {
    expect(sidebarApps(['spotify'], 1)).toEqual(['spotify'])
    expect(sidebarApps(['spotify'], 9)).toHaveLength(2)
  })

  it('remembers a music app however many others were opened since', () => {
    let state = navigate(INITIAL_NAVIGATION, { type: 'openApp', app: 'youtubeMusic' })
    for (const app of ['settings', 'youtube', 'safari', 'youtube'] as const) {
      state = navigate(state, { type: 'openApp', app })
    }
    expect(sidebarApps(state.recents, 2)).toEqual(['youtubeMusic', 'safari'])
  })
})

describe('routeKey', () => {
  it('is distinct per route', () => {
    expect(routeKey({ kind: 'home' })).toBe('home')
    expect(routeKey({ kind: 'dashboard' })).toBe('dashboard')
    expect(routeKey({ kind: 'app', app: 'youtube' })).toBe('app:youtube')
  })
})
