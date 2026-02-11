import { create } from "zustand";
import type { TrackMetadata, RepeatMode } from "../lib/types";
import * as cmd from "../lib/tauri-commands";

interface PlayerState {
  // Current track
  currentTrack: TrackMetadata | null;
  albumArt: string | null;

  // Playback
  isPlaying: boolean;
  isPaused: boolean;
  positionSecs: number;
  durationSecs: number;

  // Volume
  volume: number;
  isMuted: boolean;
  previousVolume: number;

  // Queue
  queue: TrackMetadata[];
  queueIndex: number;
  shuffle: boolean;
  repeat: RepeatMode;

  // Actions
  playTrack: (index: number) => Promise<void>;
  togglePlayPause: () => Promise<void>;
  stopPlayback: () => Promise<void>;
  seekTo: (secs: number) => Promise<void>;
  setVolume: (vol: number) => Promise<void>;
  toggleMute: () => Promise<void>;
  nextTrack: () => Promise<void>;
  previousTrack: () => Promise<void>;
  addToQueue: (tracks: TrackMetadata[]) => void;
  clearQueue: () => void;
  toggleShuffle: () => void;
  cycleRepeat: () => void;

  // Internal setters (called by useAudio hook)
  setPosition: (secs: number) => void;
  setDuration: (secs: number) => void;
  setPlaybackFlags: (playing: boolean, paused: boolean) => void;
}

export const usePlayerStore = create<PlayerState>((set, get) => ({
  // Initial state
  currentTrack: null,
  albumArt: null,
  isPlaying: false,
  isPaused: false,
  positionSecs: 0,
  durationSecs: 0,
  volume: 1.0,
  isMuted: false,
  previousVolume: 1.0,
  queue: [],
  queueIndex: -1,
  shuffle: false,
  repeat: "off",

  playTrack: async (index: number) => {
    const { queue } = get();
    if (index < 0 || index >= queue.length) return;

    const track = queue[index];
    set({
      currentTrack: track,
      queueIndex: index,
      isPlaying: true,
      isPaused: false,
      positionSecs: 0,
      durationSecs: track.duration_secs,
      albumArt: null, // Clear while loading
    });

    try {
      await cmd.playFile(track.file_path);

      // Load album art (with race condition guard)
      const art = await cmd.getAlbumArtBase64(track.file_path);
      // Verify this track is still current before setting art
      if (get().currentTrack?.file_path === track.file_path) {
        set({ albumArt: art });
      }
    } catch (e) {
      console.error("Failed to play track:", e);
    }
  },

  togglePlayPause: async () => {
    const { isPlaying, isPaused } = get();
    try {
      if (isPlaying) {
        await cmd.pause();
        set({ isPlaying: false, isPaused: true });
      } else if (isPaused) {
        await cmd.resume();
        set({ isPlaying: true, isPaused: false });
      }
    } catch (e) {
      console.error("Toggle play/pause failed:", e);
    }
  },

  stopPlayback: async () => {
    try {
      await cmd.stop();
      set({
        isPlaying: false,
        isPaused: false,
        positionSecs: 0,
        currentTrack: null,
        albumArt: null,
      });
    } catch (e) {
      console.error("Stop failed:", e);
    }
  },

  seekTo: async (secs: number) => {
    try {
      await cmd.seek(secs);
      set({ positionSecs: secs });
    } catch (e) {
      console.error("Seek failed:", e);
    }
  },

  setVolume: async (vol: number) => {
    const clamped = Math.max(0, Math.min(1, vol));
    try {
      await cmd.setVolume(clamped);
      set({ volume: clamped, isMuted: clamped === 0 });
    } catch (e) {
      console.error("Set volume failed:", e);
    }
  },

  toggleMute: async () => {
    const { isMuted, volume, previousVolume } = get();
    if (isMuted) {
      // Unmute — restore previous volume
      const restore = previousVolume > 0 ? previousVolume : 1.0;
      await cmd.setVolume(restore);
      set({ volume: restore, isMuted: false });
    } else {
      // Mute — save current volume and set to 0
      await cmd.setVolume(0);
      set({ previousVolume: volume, volume: 0, isMuted: true });
    }
  },

  nextTrack: async () => {
    const { queue, queueIndex, repeat, shuffle } = get();
    if (queue.length === 0) return;

    let nextIndex: number;

    if (repeat === "one") {
      nextIndex = queueIndex;
    } else if (shuffle) {
      // Pick random different index
      if (queue.length === 1) {
        nextIndex = 0;
      } else {
        do {
          nextIndex = Math.floor(Math.random() * queue.length);
        } while (nextIndex === queueIndex);
      }
    } else {
      nextIndex = queueIndex + 1;
      if (nextIndex >= queue.length) {
        if (repeat === "all") {
          nextIndex = 0;
        } else {
          // End of queue, stop
          set({ isPlaying: false, isPaused: false });
          return;
        }
      }
    }

    await get().playTrack(nextIndex);
  },

  previousTrack: async () => {
    const { queue, queueIndex, positionSecs } = get();
    if (queue.length === 0) return;

    // If more than 3 seconds in, restart current track
    if (positionSecs > 3) {
      await get().seekTo(0);
      return;
    }

    const prevIndex = queueIndex - 1;
    if (prevIndex >= 0) {
      await get().playTrack(prevIndex);
    } else {
      await get().seekTo(0);
    }
  },

  addToQueue: (tracks: TrackMetadata[]) => {
    set((state) => ({
      queue: [...state.queue, ...tracks],
    }));
  },

  clearQueue: () => {
    set({
      queue: [],
      queueIndex: -1,
      currentTrack: null,
      albumArt: null,
      isPlaying: false,
      isPaused: false,
    });
  },

  toggleShuffle: () => {
    set((state) => ({ shuffle: !state.shuffle }));
  },

  cycleRepeat: () => {
    set((state) => {
      const modes: RepeatMode[] = ["off", "all", "one"];
      const idx = modes.indexOf(state.repeat);
      return { repeat: modes[(idx + 1) % modes.length] };
    });
  },

  // Internal setters
  setPosition: (secs: number) => set({ positionSecs: secs }),
  setDuration: (secs: number) => set({ durationSecs: secs }),
  setPlaybackFlags: (playing: boolean, paused: boolean) =>
    set({ isPlaying: playing, isPaused: paused }),
}));
