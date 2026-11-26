import type { AppIconRole } from '../../shared/icons'

/** Every app WestPlay knows, in Home grid order. */
export const APP_IDS = [
  'music',
  'spotify',
  'youtube',
  'youtubeMusic',
  'player',
  'safari',
  'podcasts',
  'settings',
] as const

export type AppId = (typeof APP_IDS)[number]

/** An app: its id and icon (its name is in the cabin's language: state/text.ts). */
export interface AppDefinition {
  id: AppId
  icon: AppIconRole
}

export const APPS: Record<AppId, AppDefinition> = {
  music: { id: 'music', icon: 'music' },
  spotify: { id: 'spotify', icon: 'spotify' },
  youtube: { id: 'youtube', icon: 'youtube' },
  youtubeMusic: { id: 'youtubeMusic', icon: 'youtubeMusic' },
  player: { id: 'player', icon: 'player' },
  safari: { id: 'safari', icon: 'safari' },
  podcasts: { id: 'podcasts', icon: 'podcasts' },
  settings: { id: 'settings', icon: 'settings' },
}

export function isAppId(value: unknown): value is AppId {
  return typeof value === 'string' && (APP_IDS as readonly string[]).includes(value)
}

/** Not in this release: Podcasts. Its id stays (its icon, players' icon sets, the translations), but it's not on Home
 * or in the sidebar, and a saved screen or recent can't open it. (WestPlay Lite has no Maps, Phone, Messages,
 * Calendar or Weather app at all.) */
const NEXT_RELEASE: ReadonlySet<AppId> = new Set<AppId>(['podcasts'])

/** The apps this release has, in Home grid order. */
export const RELEASED_APP_IDS: readonly AppId[] = APP_IDS.filter((id) => !NEXT_RELEASE.has(id))

/** An app this release has (`isAppId`, less the ones still to come). */
export function isReleasedApp(value: unknown): value is AppId {
  return isAppId(value) && !NEXT_RELEASE.has(value)
}
