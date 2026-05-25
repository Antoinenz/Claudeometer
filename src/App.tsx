import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, emitTo } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { AuthState, Settings, UsageData, DEFAULT_SETTINGS } from "./lib/types";
import { applyDebugFlags } from "./main";
import Login from "./views/Login";
import Dashboard from "./views/Dashboard";
import Settings_ from "./views/Settings";
import Debug from "./views/Debug";

type View = "login" | "login-debug" | "dashboard" | "settings" | "debug";

const COOLDOWN_MS = 20_000;

export default function App() {
  const [view, setView] = useState<View>("login");
  const [isFocused, setIsFocused] = useState(true);
  const [simulation, setSimulation] = useState<{ usage: UsageData | null; error: string | null } | null>(null);
  const [auth, setAuth] = useState<AuthState>({ mode: "none", email: null, name: null });
  const [usage, setUsage] = useState<UsageData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [cooldownEndsAt, setCooldownEndsAt] = useState<number | null>(null);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);

  const refreshingRef = useRef(false);
  const cooldownUntilRef = useRef<number>(0);
  const cooldownTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Refs so doRefresh and the focus handler always see the current value without stale closures
  const errorRef = useRef<string | null>(null);
  const foregroundPollRef = useRef<boolean>(true);
  const authRef = useRef(auth);
  // Counts in-flight refreshes initiated by the tray (not by doRefresh itself)
  const externalRefreshCountRef = useRef(0);

  const updateError = (e: string | null) => {
    errorRef.current = e;
    setError(e);
  };

  useEffect(() => { authRef.current = auth; }, [auth]);

  useEffect(() => {
    invoke<AuthState>("get_auth_state").then((state) => {
      setAuth(state);
      setView(state.mode === "none" ? "login" : "dashboard");
      setLoading(false);
      if (state.mode !== "none") doRefresh();
    });
    invoke<Settings>("get_settings").then((s) => {
      setSettings(s);
      foregroundPollRef.current = s.foreground_poll ?? true;
    }).catch(() => {});
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Keep foregroundPollRef in sync when settings change
  useEffect(() => {
    foregroundPollRef.current = settings.foreground_poll ?? true;
  }, [settings.foreground_poll]);

  // Sync debug keyboard flags whenever settings change
  useEffect(() => {
    applyDebugFlags({
      devtools:      settings.debug_devtools,
      webviewReload: settings.debug_webview_reload,
    });
  }, [settings.debug_devtools, settings.debug_webview_reload]);

  // Background-poll events
  useEffect(() => {
    const unlistenUsage = listen<UsageData>("usage-updated", (e) => {
      setUsage(e.payload);
      updateError(null);
      // Keep auth name/email current from API response
      if (e.payload.name || e.payload.email) {
        setAuth((prev) => ({
          ...prev,
          name: e.payload.name ?? prev.name,
          email: e.payload.email ?? prev.email,
        }));
      }
    });
    const unlistenError = listen<string>("usage-error", (e) => {
      updateError(e.payload);
    });
    const unlistenTrayRefresh = listen("tray-refresh", () => {
      doRefresh();
    });
    const unlistenTrayNavigate = listen<{ view: string }>("tray-navigate", (e) => {
      if (authRef.current.mode === "none") return;
      if (e.payload.view === "settings") {
        setSimulation(null);
        emitTo("tray-menu", "simulation-cleared", {});
        setView("settings");
      } else if (e.payload.view === "dashboard") {
        setView("dashboard");
      }
    });
    // Sync spinner and cooldown with the tray menu via shared Rust events.
    // refresh-started fires for every user-triggered fetch (main app or tray).
    // We only act when doRefresh isn't already running to avoid double-setting.
    const unlistenRefreshStarted = listen("refresh-started", () => {
      if (!refreshingRef.current) {
        externalRefreshCountRef.current += 1;
        setIsRefreshing(true);
      }
    });
    const unlistenRefreshDone = listen("refresh-done", () => {
      if (externalRefreshCountRef.current > 0) {
        externalRefreshCountRef.current -= 1;
        if (externalRefreshCountRef.current === 0 && !refreshingRef.current) {
          setIsRefreshing(false);
        }
      }
    });
    // Start cooldown only when none is already running (prevents a double-reset
    // when doRefresh's finally block and this event fire close together).
    const unlistenRefreshCooldown = listen("refresh-cooldown", () => {
      if (cooldownUntilRef.current <= Date.now()) {
        if (cooldownTimerRef.current) clearTimeout(cooldownTimerRef.current);
        const endsAt = Date.now() + COOLDOWN_MS;
        cooldownUntilRef.current = endsAt;
        setCooldownEndsAt(endsAt);
        cooldownTimerRef.current = setTimeout(() => {
          cooldownUntilRef.current = 0;
          setCooldownEndsAt(null);
        }, COOLDOWN_MS);
      }
    });
    const unlistenSimStop = listen("simulation-stop-requested", () => {
      setSimulation(null);
      setView("debug");
    });
    return () => {
      unlistenUsage.then((f) => f());
      unlistenError.then((f) => f());
      unlistenTrayRefresh.then((f) => f());
      unlistenTrayNavigate.then((f) => f());
      unlistenRefreshStarted.then((f) => f());
      unlistenRefreshDone.then((f) => f());
      unlistenRefreshCooldown.then((f) => f());
      unlistenSimStop.then((f) => f());
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Hide the tray while the login-debug simulation is active; restore on exit.
  // The close handler in Rust also restores tray visibility so the user is never stranded.
  useEffect(() => {
    if (view !== "login-debug") return;
    invoke("set_tray_visible", { visible: false });
    return () => {
      invoke<Settings>("get_settings").then((s) => {
        invoke("set_tray_visible", { visible: s.show_in_tray });
      }).catch(() => {});
    };
  }, [view]); // eslint-disable-line react-hooks/exhaustive-deps

  // Ctrl+R → refresh (page reload is already blocked in main.tsx)
  useEffect(() => {
    if (auth.mode === "none") return;
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "r" && !e.shiftKey) {
        doRefresh();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [auth.mode]); // eslint-disable-line react-hooks/exhaustive-deps

  // Track focus for title bar dimming (always active)
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => setIsFocused(focused))
      .then((unlisten) => { cleanup = unlisten; });
    return () => cleanup?.();
  }, []);

  // Auto-refresh when the window comes into focus
  useEffect(() => {
    if (auth.mode === "none") return;
    let cleanup: (() => void) | undefined;
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused && !refreshingRef.current && foregroundPollRef.current) {
          doRefresh();
        }
      })
      .then((unlisten) => { cleanup = unlisten; });
    return () => cleanup?.();
  }, [auth.mode]); // eslint-disable-line react-hooks/exhaustive-deps

  const doRefresh = async () => {
    if (refreshingRef.current) return;
    // Cooldown is bypassed when there is a current error so the user can retry immediately
    if (!errorRef.current && Date.now() < cooldownUntilRef.current) return;

    refreshingRef.current = true;
    setIsRefreshing(true);
    const startedAt = Date.now();
    try {
      const d = await invoke<UsageData>("fetch_usage");
      setUsage(d);
      updateError(null);
      // Keep auth name/email current from the API response
      if (d.name || d.email) {
        setAuth((prev) => ({
          ...prev,
          name: d.name ?? prev.name,
          email: d.email ?? prev.email,
        }));
      }
    } catch (e) {
      updateError(String(e));
    } finally {
      // If the error persists (nothing changed), pad to a minimum duration so
      // the spinner doesn't flicker and the user feels confident it actually checked.
      if (errorRef.current) {
        const elapsed = Date.now() - startedAt;
        if (elapsed < 1500) await new Promise(r => setTimeout(r, 1500 - elapsed));
      }
      setIsRefreshing(false);
      refreshingRef.current = false;
      // Only apply cooldown after a successful fetch (not after errors, so retries are fast)
      if (!errorRef.current) {
        if (cooldownTimerRef.current) clearTimeout(cooldownTimerRef.current);
        const endsAt = Date.now() + COOLDOWN_MS;
        cooldownUntilRef.current = endsAt;
        setCooldownEndsAt(endsAt);
        cooldownTimerRef.current = setTimeout(() => {
          cooldownUntilRef.current = 0;
          setCooldownEndsAt(null);
        }, COOLDOWN_MS);
      }
    }
  };

  const handleLogin = (state: AuthState) => {
    setAuth(state);
    setView("dashboard");
    doRefresh();
  };

  const handleLogout = async () => {
    await invoke("logout");
    setAuth({ mode: "none", email: null, name: null });
    setUsage(null);
    updateError(null);
    setView("login");
  };

  const handleBackFromSettings = () => {
    invoke<Settings>("get_settings").then((s) => {
      setSettings(s);
      foregroundPollRef.current = s.foreground_poll ?? true;
    }).catch(() => {});
    setView("dashboard");
  };

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center bg-[#0d0d0d] border border-zinc-800/80">
        <div className="h-2 w-2 rounded-full bg-amber-500 animate-pulse" />
      </div>
    );
  }

  return (
    <div className={`h-screen bg-[#0d0d0d] flex flex-col overflow-hidden border transition-colors duration-200 ${isFocused ? "border-zinc-700/50" : "border-zinc-800/40"}`}>
      {view === "login" && <Login onLogin={handleLogin} />}
      {view === "dashboard" && (
        <Dashboard
          usage={simulation ? simulation.usage : usage}
          error={simulation ? simulation.error : error}
          isRefreshing={isRefreshing}
          cooldownEndsAt={cooldownEndsAt}
          preciseTimestamp={settings.precise_timestamp}
          hideCooldownBadge={settings.hide_cooldown_badge}
          showResetTooltip={settings.show_reset_tooltip ?? true}
          isFocused={isFocused}
          isSimulating={!!simulation}
          onSettings={() => setView("settings")}
          onRefresh={doRefresh}
          onSignOut={handleLogout}
          onStopSimulation={() => { setSimulation(null); setView("debug"); emitTo("tray-menu", "simulation-cleared", {}); }}
        />
      )}
      {view === "settings" && (
        <Settings_
          auth={auth}
          usage={usage}
          isFocused={isFocused}
          onBack={handleBackFromSettings}
          onLogout={handleLogout}
          onOpenDebug={() => setView("debug")}
        />
      )}
      {view === "debug" && (
        <Debug
          isFocused={isFocused}
          settings={settings}
          onBack={() => setView("settings")}
          onSimulate={(usage, error) => { setSimulation({ usage, error }); setView("dashboard"); emitTo("tray-menu", "simulation-set", { usage, error }); }}
          onShowLogin={() => setView("login-debug")}
          onUpdateSettings={(patch) => {
            setSettings((prev) => {
              const updated = { ...prev, ...patch };
              invoke("save_settings", { settings: updated }).catch(() => {});
              return updated;
            });
          }}
        />
      )}
      {view === "login-debug" && <Login onLogin={handleLogin} onBack={() => setView("debug")} />}
    </div>
  );
}
