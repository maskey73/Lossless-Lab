# Product Requirements Document (PRD)
## マスキー (Masukii) — Bit-Perfect Audiophile Music Player
**Version:** 0.1.0
**Date:** 2026-02-11
**Author:** maskey73

---

## 1. Executive Summary

Masukii is a desktop music player for Windows (with future cross-platform potential) targeting audiophiles who demand bit-perfect playback, WASAPI exclusive-mode output, and a modern UI rivaling services like Qobuz and Apple Music. Built as a Tauri v2 app (Rust backend + React/TypeScript frontend), it combines the power of native audio processing with a sleek, responsive interface. Think foobar2000 functionality with a Qobuz/Apple Music aesthetic.

---

## 2. Current State Analysis

### 2.1 Tech Stack
| Layer | Technology | Status |
|-------|-----------|--------|
| Runtime | Tauri v2 (Rust + WebView) | Configured |
| Frontend | React 19, TypeScript 5.9, Tailwind CSS 4 | Scaffolded (default Vite template, no player UI) |
| State | Zustand 5 | Installed, no stores created |
| UI Libraries | Framer Motion, Lucide React, React Virtuoso, TanStack Table | Installed, unused |
| Audio Decode | Symphonia (all codecs) | Working decoder with gapless support |
| Audio Output | cpal 0.15 | Working via default device output |
| Sample Rate Conversion | Rubato 0.15 | Installed, not integrated |
| Equalizer | Custom 10-band biquad | Implemented (31Hz-16kHz) |
| Metadata | Lofty 0.21 | Working (tags + album art extraction) |
| Database | rusqlite 0.32 (bundled) | Installed, placeholder only |
| File Watching | notify 7 | Installed, not integrated |
| Concurrency | crossbeam-channel, parking_lot | Integrated in audio engine |

### 2.2 What Exists (Backend — Rust)
- **Audio Engine** (`src-tauri/src/audio/engine.rs`): Fully functional command-based architecture with dedicated audio thread, decoder thread, sample buffer pipeline, volume control, pause/resume/stop/seek, and device enumeration.
- **Decoder** (`decoder.rs`): Symphonia-based decoder supporting FLAC, WAV, MP3, OGG, AAC, M4A, WMA. Handles gapless playback. Returns interleaved f32 samples.
- **Equalizer** (`equalizer.rs`): 10-band graphic EQ with biquad peaking filters. 8 built-in presets (flat, rock, pop, jazz, classical, bass_boost, vocal, electronic).
- **Metadata Reader** (`metadata/reader.rs`): Reads full tags (title, artist, album, year, genre, track/disc number, sample rate, bit depth, channels) and extracts album art as base64 data URIs.
- **Directory Scanner** (`library/scanner.rs`): Recursive audio file discovery supporting 10 formats (flac, mp3, wav, ogg, m4a, aac, wma, alac, ape, opus).
- **Tauri Commands** (`commands.rs`): 16 IPC commands exposed to frontend (playback, EQ, devices, metadata, file/folder dialogs).

### 2.3 What Exists (Frontend — React/TS)
- **Nothing functional.** The frontend is the default Vite + React template (counter button, logos). No player UI, no component architecture, no state management wired up.
- **Theme system** (`index.css`): Apple Music-inspired dark theme is defined with custom CSS variables, custom scrollbar, range slider styling, and Tauri title bar drag region support. Window configured as `decorations: false` (custom title bar required).

### 2.4 What Does NOT Exist Yet
- Any player UI (sidebar, track list, now-playing bar, album art display)
- Zustand stores (playback state, library state, playlist state, settings)
- WASAPI exclusive mode output (currently uses cpal default/shared mode)
- Library database (SQLite schema + CRUD)
- Playlist management (M3U import/export, queue)
- Settings/preferences panel
- File watcher integration for live library updates
- Rubato sample rate conversion integration
- Keyboard shortcuts / media key support
- Gapless playback at the queue/playlist level (decoder supports it, but no queue exists)

---

## 3. Product Vision

> A lightweight, beautiful, bit-perfect music player that treats audio fidelity as sacred. No resampling unless the user asks for it. WASAPI exclusive mode locks the audio device for pristine output. The UI should feel as premium as the audio quality — smooth animations, album art-centric design, and instant responsiveness even with 100k+ track libraries.

---

## 4. Feature Requirements

### Phase 1 — Core Player (MVP for EXE)
**Goal:** A working music player you can build into an .exe and use daily.

#### 4.1 Custom Window Chrome & Layout
- Custom title bar with drag region, minimize/maximize/close buttons
- Three-panel layout: Left sidebar (navigation) | Center (content area) | Bottom (now-playing bar)
- Sidebar: Library, Playlists section, Settings link
- Content area: Track list view with columns (Title, Artist, Album, Duration, Format, Sample Rate)
- Use React Virtuoso for virtualized lists (already installed)
- Use TanStack Table for sortable/resizable columns (already installed)

#### 4.2 Now-Playing Bar
- Album art thumbnail (from embedded metadata)
- Track info (title, artist, album)
- Playback controls: Previous, Play/Pause, Next, Shuffle, Repeat
- Seekbar with elapsed/remaining time display
- Volume slider with mute toggle
- Audio quality indicator badge (e.g., "FLAC 24/96" or "WAV 16/44.1")

#### 4.3 Playback Queue
- Add files via file picker dialog or folder import
- Drag-and-drop reordering
- Next/Previous track navigation
- Shuffle and repeat modes (off, repeat-all, repeat-one)
- Double-click to play from queue

#### 4.4 Zustand State Management
- `usePlayerStore`: playback state, current track, queue, position polling, volume, shuffle/repeat
- `useLibraryStore`: scanned tracks, folder paths, search/filter
- `useSettingsStore`: audio device, EQ settings, UI preferences
- Position polling via `setInterval` calling `get_position` command (~100ms interval)

#### 4.5 WASAPI Exclusive Mode (Critical Differentiator)
- Replace cpal default host with WASAPI-specific host on Windows
- Add exclusive mode toggle in settings
- When exclusive: lock audio device, output at file's native sample rate and bit depth (bit-perfect)
- When shared: use current cpal shared mode (resampled by Windows mixer)
- Display active output mode in the UI (Exclusive / Shared)
- Integrate Rubato for sample rate conversion only when in shared mode or when device doesn't support the file's native rate

#### 4.6 File/Folder Import
- "Open Files" dialog (already implemented in backend)
- "Open Folder" dialog (already implemented in backend)
- Scan folder recursively, read metadata for each file, populate track list
- Show scan progress indicator

### Phase 2 — Library & Database
**Goal:** Persistent music library with fast search and browse.

#### 4.7 SQLite Music Library
- Schema: tracks (id, path, title, artist, album, album_artist, year, genre, track_no, disc_no, duration, sample_rate, bit_depth, channels, format, file_size, last_modified, date_added)
- Add/remove library folders
- Full rescan and incremental scan (using file modification timestamps)
- Full-text search across title, artist, album
- Browse by: All Tracks, Albums, Artists, Genres

#### 4.8 Album View
- Grid layout of album covers
- Click album to expand into track listing
- Album art loaded from embedded metadata (cached in memory or disk)

#### 4.9 File Watcher
- Integrate `notify` crate to watch library folders
- Auto-detect added/removed/modified files
- Update database and UI in real-time

### Phase 3 — Playlists & Advanced Features
**Goal:** Power-user features that match foobar2000 flexibility.

#### 4.10 Playlist Management
- Create, rename, delete playlists (stored in SQLite)
- Add tracks via drag-and-drop or right-click context menu
- M3U/M3U8 import and export
- Smart playlists (auto-populate by rules: genre = "Jazz", year > 2020, etc.)

#### 4.11 Equalizer UI
- Visual 10-band EQ with draggable sliders
- Preset selector dropdown (flat, rock, pop, jazz, classical, bass_boost, vocal, electronic)
- Custom preset save/load
- Real-time frequency response curve visualization (optional)
- Enable/disable toggle

#### 4.12 Audio Device Selection
- Dropdown to select output device (backend already supports enumeration)
- Remember last-used device
- Hot-switch without stopping playback if possible

#### 4.13 Keyboard & Media Key Support
- Space: Play/Pause
- Left/Right arrows: Seek +/- 5 seconds
- Ctrl+Left/Right: Previous/Next track
- Up/Down arrows: Volume +/- 5%
- Media keys (Play, Pause, Next, Previous, Stop) via OS integration

### Phase 4 — Polish & Distribution
**Goal:** Production-ready EXE with installer.

#### 4.14 Settings Panel
- Audio: Output device, exclusive mode toggle, buffer size
- EQ: Enable/disable, preset, custom bands
- Library: Managed folders list, rescan button
- Appearance: (future) light/dark theme toggle
- About: Version, credits, links

#### 4.15 System Tray
- Minimize to tray
- Tray context menu: Play/Pause, Next, Previous, Show Window, Quit

#### 4.16 EXE Build & Installer
- Tauri bundle config already set (`"targets": "all"`)
- Windows: .exe installer (NSIS) and .msi
- Build command: `npm run tauri build`
- Code signing (optional, for distribution without SmartScreen warnings)
- Auto-updater (Tauri built-in, optional for v1)

---

## 5. Supported Audio Formats

| Format | Extension | Decode Library | Bit-Perfect | Priority |
|--------|-----------|---------------|-------------|----------|
| FLAC | .flac | Symphonia | Yes | P0 |
| WAV/PCM | .wav | Symphonia | Yes | P0 |
| ALAC | .m4a | Symphonia | Yes | P0 |
| MP3 | .mp3 | Symphonia | N/A (lossy) | P1 |
| AAC | .aac, .m4a | Symphonia | N/A (lossy) | P1 |
| OGG Vorbis | .ogg | Symphonia | N/A (lossy) | P1 |
| Opus | .opus | Symphonia | N/A (lossy) | P2 |
| WMA | .wma | Symphonia | N/A (lossy) | P2 |
| APE | .ape | Symphonia | Yes | P2 |

---

## 6. UI/UX Design Guidelines

### 6.1 Design Language
- **Dark-first** (Apple Music / Qobuz inspired — already defined in CSS theme)
- Accent color: `#fa2d48` (vibrant red, already set)
- Background hierarchy: `#0a0a0a` → `#141414` → `#1c1c1e` (depth layers)
- Typography: Inter / SF Pro Display, `-apple-system` fallback
- Smooth transitions via Framer Motion (already installed)
- Album art as visual anchor — large, prominent, high-res

### 6.2 Layout Specification
```
┌──────────────────────────────────────────────────────┐
│  [Custom Title Bar / Drag Region]          [─ □ ✕]   │
├──────────┬───────────────────────────────────────────┤
│          │                                           │
│ Sidebar  │         Content Area                      │
│          │     (Track List / Album Grid /             │
│ Library  │      Now Playing Full View)               │
│ Playlists│                                           │
│ Settings │                                           │
│          │                                           │
├──────────┴───────────────────────────────────────────┤
│ [Art] Title - Artist    [◄◄ ▶ ►►]  ───●─── 2:34/4:12│
│       Album          [🔀 🔁]  [EQ] [Vol ━━━●━] FLAC │
└──────────────────────────────────────────────────────┘
```

### 6.3 Responsiveness
- Minimum window: 900x600 (already configured in tauri.conf.json)
- Sidebar collapsible at narrow widths
- Track list columns auto-resize with window

---

## 7. Technical Architecture

### 7.1 Data Flow
```
User Action (React UI)
    ↓
Zustand Store (state update + Tauri invoke)
    ↓
Tauri IPC Command (commands.rs)
    ↓
Audio Engine (engine.rs — dedicated thread)
    ↓
Decoder Thread (symphonia) → Sample Buffer → cpal Output Stream
    ↓
WASAPI Exclusive / Shared → DAC → Audio Hardware
```

### 7.2 Threading Model
- **Main Thread**: Tauri + WebView (UI)
- **Audio Engine Thread**: Command loop, stream management
- **Decoder Thread**: Per-track, fills shared sample buffer (~2s lookahead)
- **File Scanner Thread**: Background library scanning (non-blocking)

### 7.3 WASAPI Exclusive Mode Implementation Plan
1. Use `cpal::host_from_id(cpal::HostId::Wasapi)` instead of `default_host()`
2. Query device supported configs for exact sample rate + bit depth match
3. Use `SupportedBufferSize::Range` to select optimal buffer (low latency)
4. Set exclusive mode via cpal's WASAPI-specific device config
5. If device doesn't support file's native rate → use Rubato to resample to nearest supported rate
6. Fallback to shared mode gracefully on failure

---

## 8. Build & Distribution

### 8.1 Build to EXE
```bash
# Development
npm run tauri dev

# Production build (generates .exe installer)
npm run tauri build
```

Output location: `src-tauri/target/release/bundle/`
- `nsis/マスキー_0.1.0_x64-setup.exe` (installer)
- `msi/マスキー_0.1.0_x64_en-US.msi` (MSI package)

### 8.2 Prerequisites for Building
- Node.js 18+
- Rust toolchain (rustup + stable)
- Visual Studio Build Tools (C++ workload for Windows)
- WebView2 (pre-installed on Windows 10/11)

---

## 9. Success Metrics

| Metric | Target |
|--------|--------|
| Cold start to playback | < 2 seconds |
| Track switch latency | < 200ms |
| Memory usage (idle) | < 100MB |
| Memory usage (playing) | < 200MB |
| Library scan speed | > 500 tracks/second |
| UI frame rate | 60fps constant |
| Audio output | Bit-perfect in exclusive mode (verifiable via null test) |

---

## 10. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| WASAPI exclusive mode fails on some devices | High | Graceful fallback to shared mode with user notification |
| Large libraries (100k+ tracks) cause UI lag | Medium | React Virtuoso for rendering, SQLite indexes for queries |
| Symphonia codec gaps (DSD, MQA) | Low | These are niche; document unsupported formats clearly |
| Windows SmartScreen blocks unsigned .exe | Medium | Code signing certificate or user instructions to bypass |
| cpal audio glitches under high CPU load | Medium | Increase buffer size option, priority boost for audio thread |

---

## 11. Implementation Priority

| Priority | Feature | Effort |
|----------|---------|--------|
| **P0** | Custom window chrome + title bar | 1 day |
| **P0** | Three-panel layout (sidebar, content, now-playing bar) | 2 days |
| **P0** | Zustand stores (player, library, settings) | 1 day |
| **P0** | Now-playing bar with controls + seekbar | 2 days |
| **P0** | Track list with file/folder import | 2 days |
| **P0** | WASAPI exclusive mode | 3 days |
| **P1** | SQLite library + persistent scan | 3 days |
| **P1** | Album grid view | 2 days |
| **P1** | Equalizer UI | 2 days |
| **P1** | Keyboard shortcuts + media keys | 1 day |
| **P2** | Playlist management + M3U | 3 days |
| **P2** | File watcher (live updates) | 1 day |
| **P2** | Settings panel | 2 days |
| **P2** | System tray | 1 day |
| **P3** | Auto-updater | 1 day |
| **P3** | Light theme | 1 day |

**Total estimated effort for MVP (P0): ~11 days**
**Total estimated effort for full v1 (P0-P2): ~26 days**

---

## 12. Glossary

- **Bit-Perfect**: Audio data is sent to the DAC without any modification (no resampling, no mixing, no volume processing by the OS).
- **WASAPI Exclusive Mode**: Windows Audio Session API mode that bypasses the Windows audio mixer for direct hardware access.
- **WASAPI Shared Mode**: Default Windows audio mode where audio passes through the system mixer (may resample to device's configured rate).
- **Gapless Playback**: Seamless transition between consecutive tracks with no silence gap.
- **Symphonia**: Rust-native audio decoding library supporting major codecs without external C dependencies.
- **cpal**: Cross-platform audio library for Rust that provides low-level audio I/O.
- **Rubato**: Rust library for high-quality sample rate conversion using polyphase sinc interpolation.
