import maplibregl from 'maplibre-gl'

// Limit MapLibre background tile parsing workers to 1 so the 4-core i5-8250U keeps headroom for ETS2
if (typeof maplibregl.setWorkerCount === 'function') {
  maplibregl.setWorkerCount(1)
}

/**
 * Enforces low-GPU-bandwidth MapLibre options at runtime:
 * - pixelRatio: 0.75 cuts UHD 620 fragment shader load by ~44%
 * - antialias: false disables MSAA hardware resolve overhead
 * - fadeDuration: 0 eliminates label/symbol cross-fade blending every frame
 * - maxPitch: 0 locks flat 2D view so distant perspective tiles are never fetched
 * - maxTileCacheSize: 24 keeps RAM footprint compact
 */
export function lowEndMapOptions<T extends Record<string, unknown>>(options: T): T {
  return {
    ...options,
    antialias: false,
    fadeDuration: 0,
    maxPitch: 0,
    pitchWithRotate: false,
    refreshExpiredTiles: false,
    maxTileCacheSize: 24,
    pixelRatio: 0.75,
  }
}

/**
 * Strips non-essential heavy vector layers (3D buildings, hillshades, POIs, parks, shadows)
 * so MapLibre only renders navigation roads, route lines, and essential labels.
 */
function stripHeavyLayers(map: maplibregl.Map): void {
  try {
    const style = map.getStyle()
    if (!style?.layers) return
    for (const layer of style.layers) {
      const id = layer.id.toLowerCase()
      if (
        layer.type === 'fill-extrusion' ||
        layer.type === 'hillshade' ||
        id.includes('building') ||
        id.includes('poi') ||
        id.includes('landuse') ||
        id.includes('park') ||
        id.includes('shadow')
      ) {
        if (map.getLayer(layer.id)) {
          map.setLayoutProperty(layer.id, 'visibility', 'none')
        }
      }
    }
  } catch {
    // Style may still be streaming; next styledata/load event will complete cleanup
  }
}

/**
 * Wraps a MapLibre Map instance to:
 * 1. Hide heavy vector layers as soon as style data loads.
 * 2. Convert easeTo/flyTo animations into instant jumpTo calls (eliminates 60 Hz easing CPU work).
 * 3. Throttle jumpTo camera updates to ~5.5 Hz (180 ms) with trailing final-position accuracy.
 */
export function lowEndMap(map: maplibregl.Map): maplibregl.Map {
  map.on('styledata', () => stripHeavyLayers(map))
  map.on('load', () => stripHeavyLayers(map))

  const nativeJumpTo = map.jumpTo.bind(map)
  let lastJumpAt = 0
  let pendingTimer: number | null = null
  let pendingArgs: Parameters<maplibregl.Map['jumpTo']> | null = null

  const throttledJumpTo = (...args: Parameters<maplibregl.Map['jumpTo']>): maplibregl.Map => {
    const now = performance.now()
    const elapsed = now - lastJumpAt
    if (elapsed >= 180) {
      lastJumpAt = now
      if (pendingTimer !== null) {
        window.clearTimeout(pendingTimer)
        pendingTimer = null
      }
      pendingArgs = null
      nativeJumpTo(...args)
      return map
    }

    pendingArgs = args
    if (pendingTimer === null) {
      pendingTimer = window.setTimeout(() => {
        pendingTimer = null
        lastJumpAt = performance.now()
        if (pendingArgs) {
          const latest = pendingArgs
          pendingArgs = null
          nativeJumpTo(...latest)
        }
      }, 180 - elapsed)
    }
    return map
  }

  map.jumpTo = throttledJumpTo as maplibregl.Map['jumpTo']
  map.easeTo = ((options: Parameters<maplibregl.Map['jumpTo']>[0]) =>
    throttledJumpTo(options)) as maplibregl.Map['easeTo']
  map.flyTo = ((options: Parameters<maplibregl.Map['jumpTo']>[0]) =>
    throttledJumpTo(options)) as maplibregl.Map['flyTo']

  return map
}