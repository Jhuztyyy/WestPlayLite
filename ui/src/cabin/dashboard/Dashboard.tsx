import { GuidanceCard } from '../maps/GuidanceCard'
import { useAnyWidget } from './widgetState'
import { DestinationWidget, NowPlayingWidget } from './widgets'
import './dashboard.css'

/**
 * WestPlay Lite's Dashboard: no map (it was the heaviest thing the cab drew while driving), just the column of
 * one-job widgets, centred: turn-by-turn on top while there's a route, then Now Playing and the destination.
 */
export function Dashboard() {
  const anyWidget = useAnyWidget()
  return (
    <section className="screen dashboard dashboard-lite" aria-label="Dashboard">
      <div className="widgets" hidden={!anyWidget}>
        <GuidanceCard variant="widget" />
        <NowPlayingWidget />
        <DestinationWidget />
      </div>
    </section>
  )
}
