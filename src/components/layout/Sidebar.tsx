import { Disc3, Music2, Settings } from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import { cn } from "../../lib/utils";
import type { View } from "../../lib/types";

const NAV_ITEMS: { id: View; label: string; icon: typeof Disc3 }[] = [
  { id: "now-playing", label: "Now Playing", icon: Disc3 },
  { id: "library", label: "Library", icon: Music2 },
  { id: "settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const currentView = useSettingsStore((s) => s.currentView);
  const setView = useSettingsStore((s) => s.setView);

  return (
    <div
      className="w-56 shrink-0 flex flex-col py-4 overflow-y-auto"
      style={{
        backgroundColor: "var(--color-bg-secondary)",
        borderRight: "1px solid var(--color-border-subtle)",
      }}
    >
      <div className="px-3 space-y-0.5">
        {NAV_ITEMS.map((item) => {
          const Icon = item.icon;
          const active = currentView === item.id;
          return (
            <button
              key={item.id}
              onClick={() => setView(item.id)}
              className={cn(
                "w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors",
                active && "bg-white/10",
                !active && "hover:bg-white/5"
              )}
              style={{
                color: active
                  ? "var(--color-accent)"
                  : "var(--color-text-secondary)",
              }}
            >
              <Icon size={18} />
              <span>{item.label}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
