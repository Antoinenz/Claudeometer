interface UsageBarProps {
  label: string;
  used: number | null;
  limit: number | null;
}

export default function UsageBar({ label, used, limit }: UsageBarProps) {
  const pct = used != null && limit != null && limit > 0
    ? Math.min((used / limit) * 100, 100)
    : null;

  const barColor =
    pct == null
      ? "bg-zinc-700"
      : pct >= 90
      ? "bg-red-500"
      : pct >= 75
      ? "bg-orange-500"
      : pct >= 60
      ? "bg-amber-500"
      : "bg-emerald-500";

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between text-sm">
        <span className="text-zinc-400">{label}</span>
        <span className="tabular-nums text-zinc-300 font-mono text-xs">
          {used != null && limit != null
            ? `${used.toLocaleString()} / ${limit.toLocaleString()}`
            : "—"}
        </span>
      </div>
      <div className="h-1.5 w-full rounded-full bg-zinc-800 overflow-hidden">
        <div
          className={`h-full rounded-full transition-all duration-500 ${barColor}`}
          style={{ width: pct != null ? `${pct}%` : "0%" }}
        />
      </div>
      {pct != null && (
        <p className="text-xs text-zinc-600 text-right tabular-nums">{pct.toFixed(1)}%</p>
      )}
    </div>
  );
}
