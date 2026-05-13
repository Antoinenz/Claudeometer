import { useEffect, useState } from "react";

interface UsageBarProps {
  label: string;
  utilization: number;
  resetsAt: string | null;
  showResetTooltip?: boolean;
}

function formatResetsAt(ts: string | null): string | null {
  if (!ts) return null;
  try {
    const d = new Date(ts);
    const diffMs = d.getTime() - Date.now();
    if (diffMs <= 0) return "soon";
    const h = Math.floor(diffMs / 3_600_000);
    const m = Math.floor((diffMs % 3_600_000) / 60_000);
    const days = Math.floor(h / 24);
    if (days > 0) return `${days}d ${h % 24}h`;
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  } catch {
    return null;
  }
}

function formatAbsoluteReset(ts: string | null): string | null {
  if (!ts) return null;
  try {
    const d = new Date(ts);
    if (isNaN(d.getTime()) || d.getTime() <= Date.now()) return null;

    const h = d.getHours();
    const ampm = h >= 12 ? "pm" : "am";
    const hour = h % 12 || 12;
    const min = d.getMinutes().toString().padStart(2, "0");
    const time = `${hour}:${min}${ampm}`;

    const now = new Date();
    const todayMidnight = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const resetMidnight = new Date(d.getFullYear(), d.getMonth(), d.getDate());
    const diffDays = Math.round((resetMidnight.getTime() - todayMidnight.getTime()) / 86_400_000);

    if (diffDays === 0) return `${h >= 17 ? "Tonight" : "Today"} at ${time}`;
    if (diffDays === 1) return `Tomorrow at ${time}`;
    if (diffDays <= 6) {
      const day = d.toLocaleDateString(undefined, { weekday: "long" });
      return `Next ${day} at ${time}`;
    }
    const date = d.toLocaleDateString(undefined, { month: "long", day: "numeric" });
    return `${date} at ${time}`;
  } catch {
    return null;
  }
}

const TIERS = {
  red: {
    bar:   "linear-gradient(90deg, #dc2626 0%, #f87171 100%)",
    text:  "text-red-400",
    dot:   "bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.6)]",
  },
  orange: {
    bar:   "linear-gradient(90deg, #ea580c 0%, #fb923c 100%)",
    text:  "text-orange-400",
    dot:   "bg-orange-500 shadow-[0_0_8px_rgba(249,115,22,0.5)]",
  },
  amber: {
    bar:   "linear-gradient(90deg, #d97706 0%, #fbbf24 100%)",
    text:  "text-amber-400",
    dot:   "bg-amber-500",
  },
  green: {
    bar:   "linear-gradient(90deg, #059669 0%, #34d399 100%)",
    text:  "text-emerald-400",
    dot:   "bg-emerald-500",
  },
} as const;

type Tier = keyof typeof TIERS;

function tier(pct: number): Tier {
  if (pct >= 90) return "red";
  if (pct >= 75) return "orange";
  if (pct >= 60) return "amber";
  return "green";
}

export default function UsageBar({ label, utilization, resetsAt, showResetTooltip = true }: UsageBarProps) {
  const [, setTick] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setTick(n => n + 1), 60_000);
    return () => clearInterval(id);
  }, []);

  const pct = Math.min(Math.max(utilization, 0), 100);
  const t = tier(pct);
  const colors = TIERS[t];
  const resets = formatResetsAt(resetsAt);
  const absoluteReset = formatAbsoluteReset(resetsAt);

  return (
    <div className="rounded-xl bg-zinc-900/70 border border-zinc-800/80 px-4 py-3.5 space-y-2.5">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-2 min-w-0 pt-[3px]">
          <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${colors.dot}`} />
          <span className="text-[13px] font-medium text-zinc-300 truncate">{label}</span>
          {resets && (
            <span className="relative group shrink-0 text-[11px] text-zinc-600 font-mono tabular-nums cursor-default">
              · {resets}
              {showResetTooltip && absoluteReset && (
                <span className="absolute z-50 top-full left-1/2 -translate-x-1/2 mt-2 pointer-events-none opacity-0 group-hover:opacity-100 transition-opacity duration-150">
                  <span className="absolute -top-[6px] left-1/2 -translate-x-1/2 w-0 h-0 block border-l-[6px] border-r-[6px] border-b-[6px] border-l-transparent border-r-transparent border-b-zinc-700/60" />
                  <span className="absolute -top-[4px] left-1/2 -translate-x-1/2 w-0 h-0 block border-l-[5px] border-r-[5px] border-b-[5px] border-l-transparent border-r-transparent border-b-zinc-800" />
                  <span className="block px-2.5 py-1.5 rounded-lg bg-zinc-800 border border-zinc-700/60 text-[11.5px] text-zinc-200 whitespace-nowrap shadow-xl">
                    {absoluteReset}
                  </span>
                </span>
              )}
            </span>
          )}
        </div>
        <span
          className={`text-[28px] font-medium tabular-nums leading-none ${colors.text}`}
          style={{ fontFamily: "'JetBrains Mono', monospace", letterSpacing: "-0.04em" }}
        >
          {Math.round(pct)}<span className="text-base text-zinc-600 ml-0.5">%</span>
        </span>
      </div>
      <div className="relative h-[7px] w-full rounded-full bg-zinc-800/80 overflow-hidden shadow-[inset_0_1px_1px_rgba(0,0,0,0.4)]">
        <div
          className="bar-fill relative h-full rounded-full transition-[width] duration-700 ease-out"
          style={{ width: `${pct}%`, background: colors.bar }}
        >
          <span className="bar-shine" />
        </div>
      </div>
    </div>
  );
}
