import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { AuthState, Settings, UsageData, DEFAULT_SETTINGS } from "./lib/types";
import Login from "./views/Login";
import Dashboard from "./views/Dashboard";
import Settings_ from "./views/Settings";

type View = "login" | "dashboard" | "settings";

export default function App() {
  const [view, setView] = useState<View>("login");
  const [auth, setAuth] = useState<AuthState>({ mode: "none", email: null, name: null });
  const [usage, setUsage] = useState<UsageData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [refreshCooldown, setRefreshCooldown] = useState(false);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const refreshingRef = useRef(false);
  const cooldownUntilRef = useRef<number>(0);
  const cooldownTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    invoke<AuthState>("get_auth_state").then((state) => {
      setAuth(state);
      setView(state.mode === "none" ? "login" : "dashboard");
      setLoading(false);
    });
    invoke<Settings>("get_settings").then(setSettings).catch(() => {});
  }, []);

  // Background-poll events
  useEffect(() => {
    const unlistenUsage = listen<UsageData>("usage-updated", (e) => {
      setUsage(e.payload);
      setError(null);
    });
    const unlistenError = listen<string>("usage-error", (e) => {
      setError(e.payload);
    });
    return () => {
      unlistenUsage.then((f) => f());
      unlistenError.then((f) => f());
    };
  }, []);

  // Auto-refresh when the window comes into focus
  useEffect(() => {
    if (auth.mode === "none") return;
    let cleanup: (() => void) | undefined;
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused && !refreshingRef.current) {
          doRefresh();
        }
      })
      .then((unlisten) => { cleanup = unlisten; });
    return () => cleanup?.();
  }, [auth.mode]); // eslint-disable-line react-hooks/exhaustive-deps

  const COOLDOWN_MS = 20_000;

  const doRefresh = async () => {
    if (refreshingRef.current) return;
    if (Date.now() < cooldownUntilRef.current) return;
    refreshingRef.current = true;
    setIsRefreshing(true);
    try {
      const d = await invoke<UsageData>("fetch_usage");
      setUsage(d);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsRefreshing(false);
      refreshingRef.current = false;
      if (cooldownTimerRef.current) clearTimeout(cooldownTimerRef.current);
      cooldownUntilRef.current = Date.now() + COOLDOWN_MS;
      setRefreshCooldown(true);
      cooldownTimerRef.current = setTimeout(() => {
        setRefreshCooldown(false);
        cooldownUntilRef.current = 0;
      }, COOLDOWN_MS);
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
    setError(null);
    setView("login");
  };

  const handleBackFromSettings = () => {
    // Re-fetch settings in case they changed
    invoke<Settings>("get_settings").then(setSettings).catch(() => {});
    setView("dashboard");
  };

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center bg-[#111111] border border-zinc-800/80">
        <div className="h-4 w-4 rounded-full bg-amber-600 animate-pulse" />
      </div>
    );
  }

  return (
    <div className="h-screen bg-[#111111] flex flex-col border border-zinc-800/80 overflow-hidden">
      {view === "login" && <Login onLogin={handleLogin} />}
      {view === "dashboard" && (
        <Dashboard
          usage={usage}
          error={error}
          isRefreshing={isRefreshing}
          isRefreshDisabled={isRefreshing || refreshCooldown}
          preciseTimestamp={settings.precise_timestamp}
          onSettings={() => setView("settings")}
          onRefresh={doRefresh}
        />
      )}
      {view === "settings" && (
        <Settings_
          auth={auth}
          onBack={handleBackFromSettings}
          onLogout={handleLogout}
        />
      )}
    </div>
  );
}
