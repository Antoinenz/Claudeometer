interface UsageBarProps {
  label: string;
  utilization: number;
  resetsAt: string | null;
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

const GRADIENTS = {
  red:    "linear-gradient(90deg, #dc2626 0%, #f87171 100%)",
  orange: "linear-gradient(90deg, #ea580c 0%, #fb923c 100%)",
  amber:  "linear-gradient(90deg, #d97706 0%, #fbbf24 100%)",
  green:  "linear-gradient(90deg, #059669 0%, #34d399 100%)",
} as const;

const TEXT_COLORS = {
  red:    "text-red-400",
  orange: "text-orange-400",
  amber:  "text-amber-400",
  green:  "text-emerald-400",
} as const;

type Tier = keyof typeof GRADIENTS;

function tier(pct: number): Tier {
  if (pct >= 90) return "red";
  if (pct >= 75) return "orange";
  if (pct >= 60) return "amber";
  return "green";
}

export default function UsageBar({ label, utilization, resetsAt }: UsageBarProps) {
  const pct = Math.min(Math.max(utilization, 0), 100);
  const t = tier(pct);
  const resets = formatResetsAt(resetsAt);

  return (
    <div className="rounded-xl bg-zinc-900 border border-zinc-800 px-4 py-4 space-y-3">
      <div className="flex items-baseline justify-between gap-2">
        <span className="text-xs font-semibold text-zinc-500 uppercase tracking-wider">{label}</span>
        <span className={`text-3xl font-bold tabular-nums leading-none ${TEXT_COLORS[t]}`}>
          {Math.round(pct)}<span className="text-lg font-semibold">%</span>
        </span>
      </div>
      <div className="h-3 w-full rounded-full bg-zinc-800 overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-700 ease-out"
          style={{ width: `${pct}%`, background: GRADIENTS[t] }}
        />
      </div>
      {resets && (
        <p className="text-xs text-zinc-500 text-right">resets in {resets}</p>
      )}
    </div>
  );
}
