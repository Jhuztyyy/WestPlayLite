# WestPlayLite (low-end-mode)

A lightweight, low-overhead fork of [WestPlay](https://github.com/enes-less/WestPlay) (v2.4.1) tailored for lower-end PCs and laptops running **Euro Truck Simulator 2** (and **American Truck Simulator**).

WestPlayLite strips out heavy 3D map rendering and background widget modules to reduce CPU, RAM, and WebView2 GPU usage while keeping the core CarPlay-style in-cabin infotainment experience and media playback engines intact.

---

## What Changed in low-end-mode

Compared to upstream WestPlay v2.4.1, the following resource-heavy modules were removed:
- **Map Engine & Navigation Views:** Removed MapLibre GL rendering, 3D building meshes, road geometry, traffic overlays, route search, and map dashboard views.
- **Weather Module:** Removed the Weather app, forecast polling, and dashboard weather widgets.
- **Calendar Module:** Removed the Calendar app and schedule views.

## Retained Features

- **In-Cabin Infotainment UI:** CarPlay-style touch interface rendered onto the truck dashboard GPS screen, including Home, Dashboard, App Launcher, and OEM strip modes.
- **Truck Brand Boot Sequence:** Brand-aware splash screen (Scania, DAF, Volvo, Mercedes-Benz, MAN, Renault, Iveco, and supported mod trucks) followed by the WestPlay startup animation.
- **Media & Video Playback:** Embedded WebView2 players for Spotify (with Widevine DRM and Spotify Connect), YouTube (ad-free player with captions, quality selection, and search/feed browsing), YouTube Music, and Apple Music.
- **3D Cab Acoustics & Equalizer:** Spatial speaker placement, driver head-position attenuation, exterior camera muffling, custom EQ, and live audio spectrum visualization.
- **Companion Desktop App:** Tauri-based desktop window for plugin installation, settings, keybinds, and status monitoring.
- **Multi-Language Support:** Includes 35+ localized UI language packs.

---

## Building from Source

1. Build the frontend UI:
   cd ui && npm install && npm run build && cd ..
2. Build the Rust workspace:
   cargo build --workspace --release

The compiled host and companion binary outputs to target\release\westplay.exe.

---

## Credits & License

- **Upstream Project:** [enes-less/WestPlay](https://github.com/enes-less/WestPlay) (v2.4.1)
- **Lite Fork (low-end-mode):** [Jhuztyyy/WestPlayLite](https://github.com/Jhuztyyy/WestPlayLite)
- **License:** Licensed under **GPL-3.0-or-later**. See LICENSE and THIRD_PARTY_LICENSES.md for details.
