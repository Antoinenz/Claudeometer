interface UsageBarProps {
  label: string;
  utilization: number;   // 0–100
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
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  } catch {
    return null;
  }
}

export default function UsageBar({ label, utilization, resetsAt }: UsageBarProps) {
  const pct = Math.min(Math.max(utilization, 0), 100);

  const barColor =
    pct >= 90
      ? "bg-red-500"
      : pct >= 75
      ? "bg-orange-500"
      : pct >= 60
      ? "bg-amber-500"
      : "bg-emerald-500";

  const resets = formatResetsAt(resetsAt);

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between text-sm">
        <span className="text-zinc-400">{label}</span>
        <div className="flex items-center gap-2">
          {resets && (
            <span className="text-xs text-zinc-600">resets {resets}</span>
          )}
          <span className={`tabular-nums font-mono text-xs font-medium ${
            pct >= 90 ? "text-red-400" : pct >= 75 ? "text-orange-400" : pct >= 60 ? "text-amber-400" : "text-emerald-400"
          }`}>
            {pct.toFixed(1)}%
          </span>
        </div>
      </div>
      <div className="h-1.5 w-full rounded-full bg-zinc-800 overflow-hidden">
        <div
          className={`h-full rounded-full transition-all duration-500 ${barColor}`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}
