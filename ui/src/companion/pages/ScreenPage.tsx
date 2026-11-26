import { GAME_KINDS, GAME_NAMES } from '../setup'
import { setWestplayOpen } from '../useHostStatus'
import { Group, Note, Panel, Row, Status, Switch, Tick } from '../room/parts'
import { banner } from '../room/room'
import { GLOW, useScreenAt } from '../room/screen'
import { type PageProps, screenState } from './page'
import type { CoachSpeakers } from '../useSettings'

/**
 * The truck's screen: where WestPlay is now (its headline), WestPlay on it or the truck's own, and the
 * connection from WestPlay to the game to the screen. On the room's screen, Home, and WestPlay saying the
 * same in a banner.
 */
export function ScreenPage({ status, settings, setup, cw }: PageProps) {
  const w = cw.screen
  const state = screenState(status)
  // Known once the cabin page has said (it owns the state): the switch waits for it.
  const open = status?.westplayOpen ?? null
  useScreenAt(
    state === 'original' ? 'dashboard' : 'home',
    GLOW.home,
    state === 'live'
      ? banner(w.banners.live)
      : state === 'original'
        ? banner(w.banners.original)
        : state === 'unsupported'
          ? banner(w.banners.unsupported)
          : state === 'notInCab'
            ? banner(w.banners.notInCab)
            : state === 'noGame'
              ? banner(w.banners.noGame, true)
              : null,
  )
  const fps = status?.framesPerSecond ?? 0
  // The game connected; else the games set up (else both).
  const kinds = status?.game ? [status.game] : setup.games.length ? setup.games.map((game) => game.kind) : GAME_KINDS
  const gameName = kinds.map((kind) => GAME_NAMES[kind]).join(' · ')
  return (
    <Panel place="page" title={w.titles[state]} sub={w.subs[state]}>
      <Group place={4}>
        <Row
          title={w.show}
          detail={w.showDetail}
          accessory={
            <Switch
              on={open ?? false}
              disabled={open === null || !status?.screenDrawing}
              label={w.show}
              change={(on) => {
                setWestplayOpen(on).catch((error: unknown) => console.error('WestPlay could not be switched', error))
              }}
            />
          }
        />
      </Group>
      <Group title={w.connection} place={5}>
        <Row
          title={w.engine}
          value={status?.error ? cw.values.failed : status ? w.engineRunning : w.engineStarting}
          accessory={<Status tone={status?.error ? 'bad' : status ? 'good' : 'busy'} />}
        />
        <Row
          title={gameName}
          value={status?.gameConnected ? w.gameConnected : w.gameNotRunning}
          accessory={<Status tone={status?.gameConnected ? 'good' : 'idle'} />}
        />
        <Row
          title={w.truckScreen}
          value={
            state === 'unsupported'
              ? w.screenUnsupported
              : status?.screenDrawing
                ? w.screenLive.replace('{fps}', String(fps))
                : w.screenNotInView
          }
          accessory={
            <Status
              tone={
                state === 'unsupported'
                  ? 'idle'
                  : status?.screenDrawing
                    ? 'good'
                    : status?.gameConnected
                      ? 'busy'
                      : 'idle'
              }
            />
          }
        />
      </Group>
      {status?.gameElevated && (
        <Note tone="bad">
          <b>{w.elevated.title}</b> {w.elevated.text}
        </Note>
      )}
      <GpuChooser status={status} settings={settings} cw={cw} />
      <CabSpeakersSwitch settings={settings} />
      <GameFirstSwitch settings={settings} />
      <CoachChooser status={status} settings={settings} cw={cw} />
      {status?.error && <Note tone="bad">{status.error}</Note>}
    </Panel>
  )
}

/**
 * The graphics card WestPlay's screen is drawn on: WestPlay's pick (the game's), or one the player chooses
 * when that one stutters, as iOS lists choices (a tick on the chosen, as the onboarding's game folders). The
 * one drawing it now says so; a choice starts WestPlay's screen again on that card. Only with more than one
 * card to choose from.
 */
function GpuChooser({ status, settings: state, cw }: Pick<PageProps, 'status' | 'settings' | 'cw'>) {
  const w = cw.screen
  const { settings, update } = state
  const adapters = status?.adapters ?? []
  if (!settings || adapters.length < 2) return null
  const chosen = adapters.some((adapter) => adapter.id === settings.graphics.gpu) ? settings.graphics.gpu : null
  const choose = (gpu: string | null) => {
    if (gpu !== settings.graphics.gpu) void update({ ...settings, graphics: { ...settings.graphics, gpu } })
  }
  return (
    <>
      <Group title={cw.about.gpu} place={6}>
        <Row
          title={w.gpuAuto}
          detail={w.gpuAutoDetail}
          accessory={chosen === null ? <Tick key="auto" /> : undefined}
          onClick={() => choose(null)}
        />
        {adapters.map((adapter) => (
          <Row
            key={adapter.id}
            title={adapter.name}
            detail={adapter.name === status?.cabAdapter ? w.gpuInUse : undefined}
            accessory={chosen === adapter.id ? <Tick key={adapter.id} /> : undefined}
            onClick={() => choose(adapter.id)}
          />
        ))}
      </Group>
      <Note tone="calm">{w.gpuNote}</Note>
    </>
  )
}

/**
 * WestPlay Lite: whether the music and videos are positioned on the truck's own speakers (3D sound in the cab).
 * Off keeps the sound plain and the processor free while driving. English only, as the Lite additions are.
 */
function CabSpeakersSwitch({ settings: state }: Pick<PageProps, 'settings'>) {
  const { settings, update } = state
  if (!settings) return null
  const on = settings.sound.cabSpeakers
  return (
    <>
      <Group place={7}>
        <Row
          title="Truck speakers"
          detail="Play sound through the truck’s own speakers"
          accessory={
            <Switch
              on={on}
              label="Truck speakers"
              change={(next) => void update({ ...settings, sound: { ...settings.sound, cabSpeakers: next } })}
            />
          }
        />
      </Group>
      <Note tone="calm">Off is lighter on the processor while driving; on places the sound in the cab as the game does.</Note>
    </>
  )
}

/**
 * WestPlay Lite: WestPlay at a lower Windows priority than the game, so the game keeps its frame rate when it
 * streams the map or saves, at the price of the truck screen catching up a little later then. English only.
 */
function GameFirstSwitch({ settings: state }: Pick<PageProps, 'settings'>) {
  const { settings, update } = state
  if (!settings) return null
  const on = settings.graphics.gameFirst
  return (
    <>
      <Group place={8}>
        <Row
          title="Game first"
          detail="Keep the game smooth while the map streams and it saves"
          accessory={
            <Switch
              on={on}
              label="Game first"
              change={(next) => void update({ ...settings, graphics: { ...settings.graphics, gameFirst: next } })}
            />
          }
        />
      </Group>
      <Note tone="calm">Runs WestPlay below the game in Windows. Takes full effect the next time WestPlay starts.</Note>
    </>
  )
}

/**
 * Which of a coach's speakers play (owner 2026-11-13): the driver's pair round the seat, or every unit over the
 * passengers' seats, as real coaches have a cockpit system and a passengers' one. A choice here, not in the cab; the
 * passengers' are many speakers at once, which a weak processor feels, so the note under it says how many in the coach
 * driven (its own count, from the host). Greyed out outside a coach (owner 2026-11-13).
 */
function CoachChooser({ status, settings: state, cw }: Pick<PageProps, 'status' | 'settings' | 'cw'>) {
  const w = cw.screen
  const { settings, update } = state
  if (!settings) return null
  const chosen = settings.sound.coach
  const count = status?.coachSpeakers ?? null
  const off = count === null
  const choose = (coach: CoachSpeakers) => {
    if (coach !== chosen) void update({ ...settings, sound: { ...settings.sound, coach } })
  }
  return (
    <>
      <Group title={w.coachSpeakers} place={7} disabled={off}>
        <Row
          title={w.coachDriver}
          detail={w.coachDriverDetail}
          accessory={chosen === 'driver' ? <Tick key="driver" /> : undefined}
          onClick={() => choose('driver')}
        />
        <Row
          title={w.coachPassengers}
          detail={w.coachPassengersDetail}
          accessory={chosen === 'passengers' ? <Tick key="passengers" /> : undefined}
          onClick={() => choose('passengers')}
        />
      </Group>
      <Note tone="calm" disabled={off}>
        {off ? w.coachNoteAny : w.coachNote.replace('{n}', String(count))}
      </Note>
    </>
  )
}
