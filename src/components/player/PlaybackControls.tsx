import {
  Shuffle,
  SkipBack,
  Play,
  Pause,
  SkipForward,
  Repeat,
  Repeat1,
} from "lucide-react";
import { usePlayerStore } from "../../stores/playerStore";
import { cn } from "../../lib/utils";

export function PlaybackControls() {
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const isPaused = usePlayerStore((s) => s.isPaused);
  const shuffle = usePlayerStore((s) => s.shuffle);
  const repeat = usePlayerStore((s) => s.repeat);
  const togglePlayPause = usePlayerStore((s) => s.togglePlayPause);
  const nextTrack = usePlayerStore((s) => s.nextTrack);
  const previousTrack = usePlayerStore((s) => s.previousTrack);
  const toggleShuffle = usePlayerStore((s) => s.toggleShuffle);
  const cycleRepeat = usePlayerStore((s) => s.cycleRepeat);

  const hasTrack = isPlaying || isPaused;

  return (
    <div className="flex items-center gap-2">
      {/* Shuffle */}
      <button
        onClick={toggleShuffle}
        className="w-8 h-8 flex items-center justify-center rounded-full transition-colors hover:bg-white/10"
        style={{
          color: shuffle
            ? "var(--color-accent)"
            : "var(--color-text-tertiary)",
        }}
      >
        <Shuffle size={15} />
      </button>

      {/* Previous */}
      <button
        onClick={previousTrack}
        className="w-8 h-8 flex items-center justify-center rounded-full transition-colors hover:bg-white/10"
        style={{ color: "var(--color-text-primary)" }}
      >
        <SkipBack size={18} fill="currentColor" />
      </button>

      {/* Play/Pause */}
      <button
        onClick={togglePlayPause}
        className={cn(
          "w-10 h-10 flex items-center justify-center rounded-full transition-all",
          hasTrack ? "hover:scale-105" : "hover:bg-white/10"
        )}
        style={{
          backgroundColor: hasTrack
            ? "var(--color-accent)"
            : "transparent",
          color: hasTrack ? "#fff" : "var(--color-text-primary)",
        }}
      >
        {isPlaying ? (
          <Pause size={20} fill="currentColor" />
        ) : (
          <Play size={20} fill="currentColor" className="ml-0.5" />
        )}
      </button>

      {/* Next */}
      <button
        onClick={nextTrack}
        className="w-8 h-8 flex items-center justify-center rounded-full transition-colors hover:bg-white/10"
        style={{ color: "var(--color-text-primary)" }}
      >
        <SkipForward size={18} fill="currentColor" />
      </button>

      {/* Repeat */}
      <button
        onClick={cycleRepeat}
        className="w-8 h-8 flex items-center justify-center rounded-full transition-colors hover:bg-white/10 relative"
        style={{
          color:
            repeat !== "off"
              ? "var(--color-accent)"
              : "var(--color-text-tertiary)",
        }}
      >
        {repeat === "one" ? <Repeat1 size={15} /> : <Repeat size={15} />}
        {repeat !== "off" && (
          <span
            className="absolute bottom-0.5 w-1 h-1 rounded-full"
            style={{ backgroundColor: "var(--color-accent)" }}
          />
        )}
      </button>
    </div>
  );
}
