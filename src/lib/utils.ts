/**
 * Format seconds to mm:ss or h:mm:ss display string.
 */
export function formatTime(secs: number): string {
  if (!isFinite(secs) || secs < 0) return "0:00";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  const sPad = s.toString().padStart(2, "0");
  return h > 0
    ? `${h}:${m.toString().padStart(2, "0")}:${sPad}`
    : `${m}:${sPad}`;
}

/**
 * Format audio quality badge: "FLAC 24/96" style.
 */
export function formatQuality(
  format: string,
  bitDepth: number | null,
  sampleRate: number | null
): string {
  const bd = bitDepth ?? "?";
  const sr = sampleRate
    ? sampleRate >= 1000
      ? `${(sampleRate / 1000).toFixed(sampleRate % 1000 === 0 ? 0 : 1)}`
      : `${sampleRate}`
    : "?";
  return `${format} ${bd}/${sr}`;
}

/**
 * Extract display title from metadata, falling back to filename without extension.
 */
export function displayTitle(meta: {
  title: string | null;
  file_name: string;
}): string {
  return meta.title ?? meta.file_name.replace(/\.[^.]+$/, "");
}

/**
 * Join class names, filtering out falsy values. Lightweight cn() utility.
 */
export function cn(
  ...classes: (string | false | null | undefined)[]
): string {
  return classes.filter(Boolean).join(" ");
}
