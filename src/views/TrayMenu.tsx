import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { UsageData } from "../lib/types";

type Arrow = "down" | "up";

const initialArrow: Arrow = window.location.hash === "#tray-menu-up" ? "up" : "down";

function formatReset(resetsAt: string): string {
  try {
    const diff = new Date(resetsAt).getTime() - Date.now();
    if (diff <= 0) return "now";
    const mins = Math.floor(diff / 60000);
    const hours = Math.floor(mins / 60);
    const days = Math.floor(hours / 24);
    if (days >= 1) return `${days}d`;
    if (hours >= 1) return `${hours}h`;
    return `${mins}m`;
  } catch {
    return "";
  }
}

function MiniBar({
  label,
  utilization,
  resetsAt,
}: {
  label: string;
  utilization: number;
  resetsAt: string | null;
}) {
  // The backend returns utilization already as a percentage (0–100), same as UsageBar uses.
  const pct = Math.round(Math.min(100, Math.max(0, utilization)));
  const barColor =
    pct >= 90 ? "bg-red-500"
      : pct >= 75 ? "bg-orange-500"
      : pct >= 60 ? "bg-amber-500"
      : "bg-emerald-500";
  const pctColor =
    pct >= 90 ? "text-red-400"
      : pct >= 75 ? "text-orange-400"
      : pct >= 60 ? "text-amber-400"
      : "text-emerald-400";
  const resetStr = resetsAt ? formatReset(resetsAt) : null;

  return (
    <div className="flex flex-col gap-[5px]">
      <div className="flex items-baseline gap-1.5">
        <span className="text-[10.5px] text-zinc-400 leading-none flex-1 truncate">{label}</span>
        {resetStr && (
          <span className="text-[9.5px] text-zinc-600 font-mono tabular-nums leading-none shrink-0">
            {resetStr}
          </span>
        )}
        <span className={`text-[10.5px] font-mono tabular-nums leading-none shrink-0 ${pctColor}`}>
          {pct}%
        </span>
      </div>
      <div className="relative h-[3px] bg-zinc-800 rounded-full overflow-hidden">
        <div
          className={`absolute inset-y-0 left-0 rounded-full ${barColor}`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

function ActionBtn({
  icon,
  label,
  onClick,
  danger,
  spinning,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  danger?: boolean;
  spinning?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex-1 flex flex-col items-center gap-[3px] py-1.5 rounded-md transition-colors ${
        danger
          ? "text-zinc-500 hover:text-red-400 hover:bg-red-500/10"
          : "text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800/80"
      }`}
    >
      <span
        className={`w-[14px] h-[14px] flex items-center justify-center ${spinning ? "animate-spin" : ""}`}
        style={spinning ? { animationDirection: "reverse" } : undefined}
      >
        {icon}
      </span>
      <span className="text-[9.5px] leading-none tracking-tight">{label}</span>
    </button>
  );
}

export default function TrayMenu() {
  const [arrow, setArrow] = useState<Arrow>(initialArrow);
  const [usage, setUsage] = useState<UsageData | null | "loading">("loading");
  const [refreshing, setRefreshing] = useState(false);

  useEffect(() => {
    invoke<UsageData | null>("get_cached_usage")
      .then((d) => setUsage(d))
      .catch(() => setUsage(null));

    const unlistenUsage = listen<UsageData>("usage-updated", (e) => {
      setUsage(e.payload);
    });
    const unlistenError = listen<string>("usage-error", () => {
      setUsage(null);
    });
    const unlistenOrientation = listen<{ arrow: Arrow }>("tray-menu-orientation", (e) => {
      setArrow(e.payload.arrow);
    });
    return () => {
      unlistenUsage.then((f) => f());
      unlistenError.then((f) => f());
      unlistenOrientation.then((f) => f());
    };
  }, []);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") getCurrentWindow().hide();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // For show/refresh/settings, Rust handles hiding the tray menu after the
  // main window is in front. Hiding from JS first would lose foreground and
  // prevent main from coming forward on Windows.
  const act = async (action: "show" | "refresh" | "settings" | "quit") => {
    try {
      await invoke("tray_action", { action });
    } catch (e) {
      console.error("tray_action failed", action, e);
    }
  };

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      await invoke("tray_action", { action: "refresh" });
    } catch (e) {
      console.error("refresh failed", e);
    }
    setRefreshing(false);
  };

  const arrowDown = (
    <div className="w-0 h-0 border-l-[6px] border-r-[6px] border-t-[7px] border-l-transparent border-r-transparent border-t-zinc-900 -mt-px relative z-10" />
  );
  const arrowUp = (
    <div className="w-0 h-0 border-l-[6px] border-r-[6px] border-b-[7px] border-l-transparent border-r-transparent border-b-zinc-900 -mb-px relative z-10" />
  );

  const usageData = usage !== "loading" ? usage : null;
  const isLoading = usage === "loading";
  const hasAny = usageData && (usageData.five_hour || usageData.seven_day || usageData.seven_day_sonnet);

  return (
    <div
      className={`h-screen w-screen flex flex-col items-center bg-transparent select-none px-1.5 ${
        arrow === "down" ? "pt-1.5 justify-end" : "pb-1.5 justify-start"
      }`}
    >
      {arrow === "up" && arrowUp}
      <div className="w-full bg-zinc-900 border border-zinc-800/80 rounded-lg shadow-[0_10px_28px_rgba(0,0,0,0.5)] overflow-hidden">

        {/* Header */}
        <div className="flex items-center gap-2 px-2.5 pt-2.5 pb-2">
          <img src="/icon.png" alt="" className="w-[13px] h-[13px] rounded-[3px] shrink-0" draggable={false} />
          <span className="text-[11.5px] font-semibold text-zinc-200 tracking-tight leading-none">
            Claudeometer
          </span>
        </div>

        <div className="h-px bg-zinc-800/60" />

        {/* Usage bars */}
        <div className="px-2.5 py-2 space-y-2.5">
          {isLoading && (
            <>
              <div className="h-[22px] rounded skeleton" />
              <div className="h-[22px] rounded skeleton" />
              <div className="h-[22px] rounded skeleton" />
            </>
          )}
          {!isLoading && !hasAny && (
            <div className="py-1 text-center text-[10.5px] text-zinc-600">
              {usageData === null ? "Could not load usage" : "No usage data available"}
            </div>
          )}
          {hasAny && (
            <>
              {usageData.five_hour && (
                <MiniBar
                  label="5-hour"
                  utilization={usageData.five_hour.utilization}
                  resetsAt={usageData.five_hour.resets_at}
                />
              )}
              {usageData.seven_day && (
                <MiniBar
                  label="7-day"
                  utilization={usageData.seven_day.utilization}
                  resetsAt={usageData.seven_day.resets_at}
                />
              )}
              {usageData.seven_day_sonnet && (
                <MiniBar
                  label="7d Sonnet"
                  utilization={usageData.seven_day_sonnet.utilization}
                  resetsAt={usageData.seven_day_sonnet.resets_at}
                />
              )}
            </>
          )}
        </div>

        <div className="h-px bg-zinc-800/60" />

        {/* Action buttons */}
        <div className="flex items-stretch gap-0.5 p-1">
          <ActionBtn
            icon={
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} className="w-full h-full">
                <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                <path strokeLinecap="round" strokeLinejoin="round" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
              </svg>
            }
            label="Open"
            onClick={() => act("show")}
          />
          <ActionBtn
            icon={
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} className="w-full h-full">
                <path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
            }
            label="Refresh"
            onClick={handleRefresh}
            spinning={refreshing}
          />
          <ActionBtn
            icon={
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} className="w-full h-full">
                <path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
            }
            label="Settings"
            onClick={() => act("settings")}
          />
          <ActionBtn
            icon={
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} className="w-full h-full">
                <path strokeLinecap="round" strokeLinejoin="round" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
              </svg>
            }
            label="Quit"
            onClick={() => act("quit")}
            danger
          />
        </div>
      </div>
      {arrow === "down" && arrowDown}
    </div>
  );
}
