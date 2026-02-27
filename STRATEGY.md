# Masukii Fix Strategy: From Prototype to Product

## Diagnosis

### What Went Wrong

The backend audio engine is genuinely **professional-grade** — bit-perfect playback, lock-free ring buffers, equal-power cosine fades, ReplayGain with clipping prevention, a null test for verifying decoder consistency. This is foobar2000/Qobuz-level audio engineering.

But the frontend is an **unfinished prototype**. Core features are missing or stubbed out, making the app feel broken even though the underlying code quality is actually solid.

### The Real Problem: Incomplete, Not Broken

| Issue | Details |
|-------|---------|
| "Open Folder" does nothing | Opens a dialog, gets the path, then throws it away (`NowPlaying.tsx:35-47`). The scanner exists in Rust and works — it was just never wired up. |
| Library says "coming soon" | The entire Library view is a single line of placeholder text (`App.tsx:32-40`). |
| No queue view | Tracks get queued internally but users can't see, reorder, or remove them. The queue logic in `playerStore.ts` is well-implemented but invisible. |
| No keyboard shortcuts | Not even spacebar for play/pause. |
| No drag & drop | The only way to open music is through system file dialogs. |
| Polling bug in useAudio | `queue` in the useEffect dependency array (`useAudio.ts:51`) causes the polling interval to restart every time the queue changes. |
| Errors invisible to users | All errors go to `console.error` — users see nothing when things fail. |
| Sparse UI | No album art backgrounds, no view transitions, no loading indicators. Looks like a dev mockup. |

### What's Actually Good (Preserve This)

- Audio engine: bit-perfect signal path, lock-free SPSC ring buffer (131072 samples), equal-power fade state machine, hard limiter only when needed
- Tauri command integration: all 21 commands properly registered and type-safe
- Zustand stores: clean design, proper separation of concerns
- CSS/Tailwind theme: solid Apple Music-style dark theme foundation
- Component architecture: clean imports, proper TypeScript throughout

### Dependencies Already Installed But Unused

These are already in `package.json` / `Cargo.toml` — ready to use:

| Dependency | Purpose | Status |
|-----------|---------|--------|
| `react-virtuoso` | Virtualized lists for queue/library | npm installed, unused |
| `@tanstack/react-table` | Sortable/filterable tables for library | npm installed, unused |
| `framer-motion` | Animations (only used minimally in NowPlaying) | npm installed, underused |
| `rusqlite` | SQLite for library database | In Cargo.toml, unused |
| `notify` | File system watcher for auto-refresh | In Cargo.toml, unused |
| `rubato` | Audio resampling | In Cargo.toml, unused |

---

## Fix Strategy: 3 Phases

### Phase 1: Make It Functional (CRITICAL)

**Goal: The app does what a music player should do.**

#### 1.1 Wire Up "Open Folder"

The scanner (`src-tauri/src/library/scanner.rs`) already recursively scans directories for audio files and returns sorted paths. It just needs a Tauri command to expose it to the frontend.

**Backend:**
- Add `scan_folder` command to `commands.rs` — calls existing `scanner::scan_directory()` + reads metadata for each file
- Register in `lib.rs` `generate_handler!`

**Frontend:**
- Add `scanFolder` binding in `tauri-commands.ts`
- Fix `handleOpenFolder` in `NowPlaying.tsx` to actually scan, queue tracks, and start playing

#### 1.2 Fix the useAudio Polling Bug

**File:** `src/hooks/useAudio.ts`

The `queue` variable is in the useEffect dependency array (line 51). Every time a track is added to the queue, the polling interval tears down and recreates — causing jank and potentially skipping auto-advance detection.

**Fix:** Use a `useRef` for the queue value instead of including it in the dependency array.

#### 1.3 Add Keyboard Shortcuts

**New file:** `src/hooks/useKeyboardShortcuts.ts`

| Key | Action |
|-----|--------|
| Space | Play/Pause |
| Left Arrow | Seek back 5s |
| Right Arrow | Seek forward 5s |
| Ctrl+Left | Previous track |
| Ctrl+Right | Next track |
| Up Arrow | Volume up 5% |
| Down Arrow | Volume down 5% |
| M | Toggle mute |

Mount in `App.tsx` alongside `useAudio()`.

#### 1.4 Add Drag & Drop Support

**New file:** `src/hooks/useDragDrop.ts`

Use Tauri's `onDragDropEvent` API. When audio files are dropped:
- Filter for supported extensions (.flac, .mp3, .wav, .ogg, .m4a, etc.)
- Read metadata for each file
- Add to queue
- Auto-start playback if nothing is playing

#### 1.5 Build the Queue View

**New file:** `src/components/player/QueueView.tsx`

Currently, tracks can be queued but there's no UI for it. Build a view showing:
- All queued tracks (using `react-virtuoso` for performance)
- Current track highlighted
- Click to jump to any track
- X button to remove a track
- Track count header

**Also modify:**
- `types.ts`: add `"queue"` to the `View` type
- `Sidebar.tsx`: add Queue navigation item
- `App.tsx`: add queue view routing
- `playerStore.ts`: add `removeFromQueue` action

---

### Phase 2: Make It Look Good (HIGH)

**Goal: The app looks polished, not like a developer prototype.**

#### 2.1 Improve NowPlaying View

**Empty state:**
- Replace grey box with an inviting design
- Add drop zone hint text ("drag files here or...")
- More elegant icon arrangement

**Playing state:**
- Add blurred album art background behind main art (like Apple Music / Spotify)
- CSS: `filter: blur(80px) saturate(1.5); opacity: 0.25;`
- Smooth transition when track changes

#### 2.2 Enhance the Sidebar

- Add "Open Files" and "Open Folder" quick-access buttons (persistent access, not just in NowPlaying empty state)
- Add mini now-playing info at bottom of sidebar (small album art + track title + artist)
- Provides context regardless of which view the user is on

#### 2.3 Toast Notifications for Errors/Loading

**New files:** `src/stores/uiStore.ts`, `src/components/ui/Toast.tsx`

- Replace all `console.error` calls with visible toast notifications
- Add loading spinners for long operations (folder scanning)
- Toasts auto-dismiss after 4 seconds
- Use framer-motion for enter/exit animations

#### 2.4 Polish Seekbar and Volume Slider

- Make slider thumb always visible (currently hidden until hover — confusing)
- Add hover-expand effect (bar grows from 4px to 6px on hover)
- Add hover time preview tooltip on seekbar
- Wider click target for easier seeking

#### 2.5 View Transition Animations

- Wrap views in `AnimatePresence` from framer-motion (already installed)
- Smooth 150ms fade + subtle vertical shift between views
- Prevents jarring instant view switches

---

### Phase 3: Build the Library (MEDIUM)

**Goal: Real music collection management with persistent storage.**

#### 3.1 SQLite Library Database

**File:** `src-tauri/src/library/database.rs` (currently empty stub)

Using `rusqlite` (already in Cargo.toml):

**Schema:**
- `tracks` table: file_path (unique), title, artist, album, album_artist, year, genre, track_number, disc_number, duration_secs, sample_rate, bit_depth, channels, file_name, format, has_album_art, folder_path, added_at
- `library_folders` table: path (primary key), added_at

**Operations:** insert tracks, get all, search by text, get albums, get artists, remove by folder

#### 3.2 Library Tauri Commands

Add to `commands.rs`:
- `add_library_folder` — scans folder + stores in DB
- `get_library_tracks` — returns all tracks from DB
- `search_library` — text search across title/artist/album
- `get_library_folders` — returns managed folder list
- `remove_library_folder` — removes folder and its tracks from DB

Add `LibraryDB` to `AppState` in `lib.rs`.

#### 3.3 Library View (Frontend)

**New files:** `src/components/library/LibraryView.tsx`, `src/stores/libraryStore.ts`

A sortable, searchable track table using `@tanstack/react-table` + `react-virtuoso`:

```
+---------------------------------------------------+
| Library                    [Search...] [Add Folder] |
+---------------------------------------------------+
| #  | Title        | Artist    | Album    | Duration |
| 1  | Track Name   | Artist    | Album    | 3:42     |
| 2  | Track Name   | Artist    | Album    | 4:15     |
| ...virtualized rows...                              |
+---------------------------------------------------+
```

Features: column sorting, live search filter, double-click to play, right-click context menu, persists across app restarts.

---

## File Change Summary

### Files to Modify

| File | Phases |
|------|--------|
| `src-tauri/src/commands.rs` | 1.1, 3.2 |
| `src-tauri/src/lib.rs` | 1.1, 3.2 |
| `src-tauri/src/library/database.rs` | 3.1 |
| `src/App.tsx` | 1.3, 1.4, 1.5, 2.5 |
| `src/hooks/useAudio.ts` | 1.2 |
| `src/components/player/NowPlaying.tsx` | 1.1, 2.1 |
| `src/components/layout/Sidebar.tsx` | 1.5, 2.2 |
| `src/components/layout/BottomBar.tsx` | 2.4 |
| `src/stores/playerStore.ts` | 1.5 |
| `src/lib/tauri-commands.ts` | 1.1, 3.4 |
| `src/lib/types.ts` | 1.5 |
| `src/index.css` | 2.1, 2.4 |

### New Files to Create

| File | Phase | Purpose |
|------|-------|---------|
| `src/hooks/useKeyboardShortcuts.ts` | 1.3 | Global keyboard shortcut handler |
| `src/hooks/useDragDrop.ts` | 1.4 | Tauri drag & drop event handler |
| `src/components/player/QueueView.tsx` | 1.5 | Queue list with react-virtuoso |
| `src/stores/uiStore.ts` | 2.3 | Toast notification state |
| `src/components/ui/Toast.tsx` | 2.3 | Toast notification component |
| `src/components/library/LibraryView.tsx` | 3.3 | Library table view |
| `src/stores/libraryStore.ts` | 3.3 | Library state management |

---

## Verification Checklist

### Phase 1
- [ ] Click "Open Folder" > select a music directory > tracks load into queue and first track plays
- [ ] Press spacebar > playback pauses/resumes
- [ ] Press arrow keys > seek forward/back 5 seconds
- [ ] Drag .flac files from Explorer onto window > tracks queue and play
- [ ] Navigate to Queue view > see all tracks, click to jump, X to remove

### Phase 2
- [ ] Play a track with album art > background shows blurred art
- [ ] Trigger an error > toast notification appears in bottom-right
- [ ] Switch views > smooth animated transition
- [ ] Hover seekbar > time preview tooltip appears
- [ ] Sidebar shows mini now-playing info at bottom

### Phase 3
- [ ] Library view shows sortable table of all scanned tracks
- [ ] Type in search box > table filters live
- [ ] Double-click a track > starts playing
- [ ] Close and reopen app > library persists
- [ ] Add/remove library folders > tracks update accordingly
