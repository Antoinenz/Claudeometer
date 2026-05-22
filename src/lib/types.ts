export interface UsageWindow {
  utilization: number;
  resets_at: string | null;
}

export interface UsageData {
  five_hour: UsageWindow | null;
  seven_day: UsageWindow | null;
  seven_day_sonnet: UsageWindow | null;
  org_name: string | null;
  name: string | null;
  email: string | null;
  tier: string;
  fetched_at: string;
  source: string;
}

export interface AuthState {
  mode: "none" | "session_key";
  email: string | null;
  name: string | null;
}

// Discriminated union — matches the Rust NotificationRule serde tag
export type NotificationRule =
  | { type: "threshold"; id: string; window: string; at_pct: number }
  | { type: "spike";     id: string; window: string; by_pct: number }
  | { type: "reset_soon"; id: string; window: string; within_mins: number }
  | { type: "recovery";  id: string; window: string; below_pct: number };

export interface Settings {
  launch_at_startup: boolean;
  show_in_tray: boolean;
  minimize_to_tray: boolean;
  notifications_enabled: boolean;
  notification_rules: NotificationRule[];
  ntfy_enabled: boolean;
  ntfy_server: string;
  ntfy_topic: string;
  ntfy_rules: NotificationRule[];
  poll_interval_secs: number;
  precise_timestamp: boolean;
  hide_cooldown_badge: boolean;
  show_reset_tooltip: boolean;
  debug_devtools: boolean;
  debug_webview_reload: boolean;
  auto_poll: boolean;
  foreground_poll: boolean;
  api_enabled: boolean;
  api_port: number;
  api_local_only: boolean;
  api_require_auth: boolean;
  api_key: string;
  api_allow_read_usage: boolean;
  api_allow_refresh: boolean;
  api_allow_read_settings: boolean;
  api_allow_write_settings: boolean;
}

export const DEFAULT_SETTINGS: Settings = {
  launch_at_startup: false,
  show_in_tray: true,
  minimize_to_tray: true,
  notifications_enabled: false,
  notification_rules: [],
  ntfy_enabled: false,
  ntfy_server: "https://ntfy.sh",
  ntfy_topic: "claudeometer",
  ntfy_rules: [],
  poll_interval_secs: 60,
  precise_timestamp: false,
  hide_cooldown_badge: false,
  show_reset_tooltip: true,
  debug_devtools: false,
  debug_webview_reload: false,
  auto_poll: true,
  foreground_poll: true,
  api_enabled: false,
  api_port: 7842,
  api_local_only: true,
  api_require_auth: true,
  api_key: "",
  api_allow_read_usage: true,
  api_allow_refresh: false,
  api_allow_read_settings: false,
  api_allow_write_settings: false,
};
