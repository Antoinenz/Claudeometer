import { AuthState, UsageData } from "../lib/types";
import UsageBar from "../components/UsageBar";

interface Props {
  auth: AuthState;
  usage: UsageData | null;
  error: string | null;
  onSettings: () => void;
  onRefresh: () => void;
}

function formatResetAt(resetAt: string | null): string | null {
  if (!resetAt) return null;
  try {
    const d = new Date(resetAt);
    const now = new Date();
    const diffMs = d.getTime() - now.getTime();
    if (diffMs <= 0) return "soon";
    const diffH = Math.floor(diffMs / 3600000);
    const diffM = Math.floor((diffMs % 3600000) / 60000);
    if (diffH > 0) return `${diffH}h ${diffM}m`;
    return `${diffM}m`;
  } catch {
    return null;
  }
}

function formatFetchedAt(ts: string): string {
  try {
    return new Date(ts).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return ts;
  }
}

export default function Dashboard({ auth, usage, error, onSettings, onRefresh }: Props) {
  const resetsIn = formatResetAt(usage?.reset_at ?? null);
  const hasLimitData = usage?.messages_limit != null;

  return (
    <div className="flex flex-col h-full">
      {/* Topbar */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-zinc-800/60">
        <div className="flex items-center gap-2">
          <span className="text-base">⊙</span>
          <span className="text-sm font-medium text-zinc-200">Claudeometer</span>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={onRefresh}
            title="Refresh"
            className="p-1.5 rounded-md text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800 transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
          </button>
          <button
            onClick={onSettings}
            title="Settings"
            className="p-1.5 rounded-md text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800 transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-5 py-5 space-y-6">
        {/* Account card */}
        <div className="rounded-xl bg-zinc-900 border border-zinc-800 px-4 py-3 flex items-center gap-3">
          <div className="w-2 h-2 rounded-full bg-emerald-400 shrink-0" />
          <div className="min-w-0">
            <p className="text-sm text-zinc-200 truncate">
              {auth.email ?? (auth.mode === "api_key" ? "API Key" : "Connected")}
            </p>
            {usage?.plan && (
              <p className="text-xs text-zinc-600 capitalize mt-0.5">{usage.plan} plan</p>
            )}
          </div>
        </div>

        {/* Error state */}
        {error && (
          <div className="rounded-xl bg-red-500/5 border border-red-500/20 px-4 py-3">
            <p className="text-xs text-red-400">{error}</p>
          </div>
        )}

        {/* Usage section */}
        {!usage && !error && (
          <div className="rounded-xl bg-zinc-900 border border-zinc-800 px-4 py-8 text-center">
            <p className="text-sm text-zinc-600">Fetching usage…</p>
          </div>
        )}

        {usage && (
          <div className="rounded-xl bg-zinc-900 border border-zinc-800 px-4 py-4 space-y-5">
            <div className="flex items-center justify-between">
              <h2 className="text-xs font-medium text-zinc-500 uppercase tracking-wider">Usage</h2>
              {resetsIn && (
                <span className="text-xs text-zinc-600">
                  Resets in <span className="text-zinc-400">{resetsIn}</span>
                </span>
              )}
            </div>

            {hasLimitData ? (
              <UsageBar
                label="Messages"
                used={usage.messages_used}
                limit={usage.messages_limit}
              />
            ) : (
              <div className="text-center py-4">
                <p className="text-sm text-zinc-600">
                  {usage.source === "api_key"
                    ? "Detailed usage not available via API key."
                    : "Usage data not returned by Claude.ai."}
                </p>
                <p className="text-xs text-zinc-700 mt-1">
                  {usage.source === "api_key"
                    ? "Switch to session key mode for limit tracking."
                    : "Claude.ai may not expose limits for your plan."}
                </p>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Footer */}
      {usage && (
        <div className="px-5 py-3 border-t border-zinc-800/60">
          <p className="text-xs text-zinc-700 text-center">
            Updated {formatFetchedAt(usage.fetched_at)}
          </p>
        </div>
      )}
    </div>
  );
}
