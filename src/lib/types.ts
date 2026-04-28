export interface UsageData {
  messages_used: number | null;
  messages_limit: number | null;
  reset_at: string | null;
  plan: string | null;
  org_name: string | null;
  email: string | null;
  fetched_at: string;
  source: string;
}

export interface AuthState {
  mode: "none" | "session_key" | "api_key";
  email: string | null;
}

export interface Settings {
  launch_at_startup: boolean;
  minimize_to_tray: boolean;
  desktop_notifications: boolean;
  notification_threshold: number;
  poll_interval_secs: number;
  ntfy_enabled: boolean;
  ntfy_server: string;
  ntfy_topic: string;
}

export const DEFAULT_SETTINGS: Settings = {
  launch_at_startup: false,
  minimize_to_tray: true,
  desktop_notifications: true,
  notification_threshold: 80,
  poll_interval_secs: 60,
  ntfy_enabled: false,
  ntfy_server: "https://ntfy.sh",
  ntfy_topic: "claudeometer",
};
