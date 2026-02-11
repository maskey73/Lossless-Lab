import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";

export function TitleBar() {
  const appWindow = getCurrentWindow();

  return (
    <div
      data-tauri-drag-region
      className="h-9 flex items-center justify-between select-none shrink-0"
      style={{ backgroundColor: "var(--color-bg-secondary)", borderBottom: "1px solid var(--color-border-subtle)" }}
    >
      {/* App title */}
      <div
        data-tauri-drag-region
        className="pl-4 text-sm font-medium tracking-wide"
        style={{ color: "var(--color-text-secondary)" }}
      >
        マスキー
      </div>

      {/* Window controls */}
      <div className="flex h-full">
        <button
          onClick={() => appWindow.minimize()}
          className="w-[46px] h-full flex items-center justify-center transition-colors hover:bg-white/10"
          style={{ color: "var(--color-text-secondary)" }}
        >
          <Minus size={14} />
        </button>
        <button
          onClick={() => appWindow.toggleMaximize()}
          className="w-[46px] h-full flex items-center justify-center transition-colors hover:bg-white/10"
          style={{ color: "var(--color-text-secondary)" }}
        >
          <Square size={11} />
        </button>
        <button
          onClick={() => appWindow.close()}
          className="w-[46px] h-full flex items-center justify-center transition-colors hover:bg-red-600"
          style={{ color: "var(--color-text-secondary)" }}
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
