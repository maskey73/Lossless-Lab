import { Volume, Volume1, Volume2, VolumeX } from "lucide-react";
import { usePlayerStore } from "../../stores/playerStore";

export function VolumeControl() {
  const volume = usePlayerStore((s) => s.volume);
  const isMuted = usePlayerStore((s) => s.isMuted);
  const setVolume = usePlayerStore((s) => s.setVolume);
  const toggleMute = usePlayerStore((s) => s.toggleMute);

  const VolumeIcon =
    isMuted || volume === 0
      ? VolumeX
      : volume < 0.33
        ? Volume
        : volume < 0.66
          ? Volume1
          : Volume2;

  return (
    <div className="flex items-center gap-2">
      <button
        onClick={toggleMute}
        className="w-8 h-8 flex items-center justify-center rounded-full transition-colors hover:bg-white/10"
        style={{ color: "var(--color-text-secondary)" }}
      >
        <VolumeIcon size={16} />
      </button>
      <input
        type="range"
        min={0}
        max={100}
        value={Math.round(volume * 100)}
        onChange={(e) => setVolume(Number(e.target.value) / 100)}
        className="w-24 h-1 rounded-full appearance-none cursor-pointer"
        style={{
          background: `linear-gradient(to right, var(--color-accent) ${volume * 100}%, var(--color-bg-hover) ${volume * 100}%)`,
        }}
      />
    </div>
  );
}
