import { useCabin } from '../state/store'
import { NowPlayingWidget } from './widgets'
import './dashboard.css'

/**
 * WestPlay Lite's Dashboard: no map, no turn-by-turn and no destination (the game's own GPS does those), just Now
 * Playing, centred.
 */
export function Dashboard() {
  const nowPlaying = useCabin((state) => state.dashboardWidgets.nowPlaying)
  return (
    <section className="screen dashboard dashboard-lite" aria-label="Dashboard">
      <div className="widgets" hidden={!nowPlaying}>
        <NowPlayingWidget />
      </div>
    </section>
  )
}
