import { useEffect, useRef } from "react";
import { usePlayerStore } from "../stores/playerStore";
import * as cmd from "../lib/tauri-commands";

/**
 * useAudio — polls the Rust backend for playback state at ~100ms intervals.
 * Mount this ONCE in App.tsx. It keeps the playerStore in sync with the
 * audio engine's actual state.
 *
 * Also auto-advances to next track when current track ends naturally.
 */
export function useAudio() {
  const intervalRef = useRef<number | null>(null);
  const wasPlayingRef = useRef(false);

  const setPosition = usePlayerStore((s) => s.setPosition);
  const setDuration = usePlayerStore((s) => s.setDuration);
  const setPlaybackFlags = usePlayerStore((s) => s.setPlaybackFlags);
  const nextTrack = usePlayerStore((s) => s.nextTrack);
  const queue = usePlayerStore((s) => s.queue);

  useEffect(() => {
    const poll = async () => {
      try {
        const state = await cmd.getPlaybackState();
        setPosition(state.position_secs);
        setDuration(state.duration_secs);
        setPlaybackFlags(state.is_playing, state.is_paused);

        // Auto-advance: if was playing and now stopped (not paused), go to next
        if (
          wasPlayingRef.current &&
          !state.is_playing &&
          !state.is_paused &&
          queue.length > 0
        ) {
          nextTrack();
        }
        wasPlayingRef.current = state.is_playing;
      } catch {
        // Backend not ready — ignore
      }
    };

    intervalRef.current = window.setInterval(poll, 100);
    return () => {
      if (intervalRef.current !== null) {
        window.clearInterval(intervalRef.current);
      }
    };
  }, [setPosition, setDuration, setPlaybackFlags, nextTrack, queue]);
}
