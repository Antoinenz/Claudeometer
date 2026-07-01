# Claudeometer — Developer Documentation

## Table of Contents

1. [Architecture overview](#architecture-overview)
2. [Project structure](#project-structure)
3. [Building and running](#building-and-running)
4. [Data flow](#data-flow)
5. [Settings system](#settings-system)
6. [Tauri event bus](#tauri-event-bus)
7. [Tray menu](#tray-menu)
8. [Notification rules](#notification-rules)
9. [HTTP API server](#http-api-server)
10. [Adding new features](#adding-new-features)

---

## Architecture overview

The repo is a Cargo workspace with three Rust crates:

- `crates/core` (`claudeometer-core`) — the claude.ai HTTP client (`claude.rs`) and shared `UsageData`/`UsageWindow` types. No Tauri dependency; used by both of the below.
- `src-tauri` (`claudeometer`) — the Tauri v2 desktop app this file documents.
- `crates/service` (`claudeometer-service`) — the headless, GUI-free service binary. See [docs/SERVICE.md](SERVICE.md) instead of this file if that's what you're working on.

Claudeometer (the desktop app) has two logical layers:

```
┌─────────────────────────────────────────────────────┐
│  Rust process (src-tauri/)                          │
│                                                     │
│  ┌──────────────┐   ┌────────────┐  ┌────────────┐  │
│  │  lib.rs      │   │commands.rs │  │  api.rs    │  │
│  │  (setup,     │◄──│  (Tauri    │  │  (axum     │  │
│  │   tray,      │   │   commands)│  │   HTTP     │  │
│  │   polling,   │   └────────────┘  │   server)  │  │
│  │   notifs)    │                   └────────────┘  │
│  └──────┬───────┘                                   │
│         │  Tauri IPC / events                       │
└─────────┼───────────────────────────────────────────┘
          │
┌─────────┼───────────────────────────────────────────┐
│  WebView (src/)          │                          │
│                          ▼                          │
│  ┌───────────┐   ┌───────────────┐                  │
│  │  App.tsx  │   │  TrayMenu.tsx │                  │
│  │  (main    │   │  (tray popup  │                  │
│  │   window) │   │   window)     │                  │
│  └───────────┘   └───────────────┘                  │
└─────────────────────────────────────────────────────┘
```

- **Rust** handles all I/O: HTTP requests to Claude.ai, OS keychain, file store, tray icon, background polling, notification dispatch, and the optional API server.
- **WebView** is a React SPA. Two independent instances exist: `main` (the app window) and `tray-menu` (the popup). They share no state directly — all synchronisation goes through Tauri events.
- **Tauri commands** (`invoke(...)` from JS) are the RPC layer from WebView to Rust.
- **Tauri events** (`emit`/`listen`) are the broadcast layer for pushing updates in both directions.

---

## Project structure

```
claudeometer/
├── Cargo.toml                   # Workspace root: members = crates/core, crates/service, src-tauri
├── src/                         # React/TypeScript frontend (desktop app only)
│   ├── App.tsx                  # Root component, routing, event listeners
│   ├── main.tsx                 # Vite entry point
│   ├── index.css                # Global styles (Tailwind)
│   ├── components/
│   │   ├── UsageBar.tsx         # Single usage window bar + tooltip
│   │   └── WindowControls.tsx   # Custom title-bar close/minimise buttons
│   ├── views/
│   │   ├── Dashboard.tsx        # Main usage view
│   │   ├── Settings.tsx         # Settings page
│   │   ├── Login.tsx            # Session-key input
│   │   ├── TrayMenu.tsx         # Tray popup UI
│   │   └── Debug.tsx            # Hidden debug panel (tap logo ×5)
│   └── lib/
│       └── types.ts             # Shared TypeScript types + DEFAULT_SETTINGS
│
├── crates/
│   ├── core/                    # claudeometer-core: shared claude.ai client
│   │   └── src/
│   │       ├── lib.rs
│   │       └── claude.rs        # Claude.ai HTTP client (moved from src-tauri)
│   └── service/                 # claudeometer-service: headless binary — see docs/SERVICE.md
│       └── src/
│           ├── main.rs          # clap CLI
│           ├── config.rs        # config file + keyring-or-file credential storage
│           ├── api.rs           # axum HTTP API
│           ├── poller.rs        # background polling loop
│           ├── run.rs           # shared foreground run path
│           └── svc/             # self-install as systemd/launchd/Windows service
│
├── src-tauri/
│   ├── Cargo.toml               # Rust dependencies (desktop app)
│   ├── tauri.conf.json          # Tauri app config (windows, bundle, CSP)
│   ├── icons/                   # App icons for all platforms
│   └── src/
│       ├── lib.rs               # App setup: tray, polling, notifications, close behaviour
│       ├── commands.rs          # All #[tauri::command] handlers + Settings struct
│       └── api.rs               # axum HTTP API server (desktop app's own, separate from the service's)
│
├── scripts/
│   └── install.sh                # curl|sh installer for claudeometer-service
├── .github/workflows/
│   ├── release.yml               # Desktop app release CI (Windows/macOS/Linux installers)
│   └── release-service.yml       # Headless service release CI (cross-platform binaries)
├── docs/
│   ├── DEVELOPER.md              # This file (desktop app)
│   └── SERVICE.md                # Headless service reference
└── README.md
```

---

## Building and running

### Prerequisites

| Tool | Version |
|------|---------|
| Rust | stable (≥ 1.77) |
| Node.js | LTS |
| WebView2 runtime | Windows only — ships with Windows 11, available separately for Windows 10 |

### Dev mode

```bash
npm install
npm run tauri dev
```

Vite starts on `http://localhost:1420` with HMR. Tauri hot-reloads the WebView on file changes. Rust changes require a full recompile (Tauri does this automatically).

### Production build

```bash
npm run tauri build
```

Outputs installers to `src-tauri/target/release/bundle/`.

### Release CI

Pushing a `v*` tag triggers `.github/workflows/release.yml`, which builds for Windows (x86_64), macOS (universal), and Linux (x86_64) in parallel and creates a draft GitHub release with all installers attached.

```bash
git tag -a v1.2.3 -m "v1.2.3"
git push origin v1.2.3
```

Tags containing `alpha`, `beta`, or `rc` are automatically marked as pre-releases.

---

## Data flow

### Fetching usage

```
JS: invoke("fetch_usage")
  → commands.rs: fetch_usage()
      emit("refresh-started")        ← both windows show spinner
      do_fetch_usage()
        → claude.rs: fetch_claude_usage(session_key)
            GET /api/organizations
            GET /api/organizations/{id}/usage   ┐ parallel
            GET /api/account                    ┘
        → UsageCache (managed state) updated
      emit("refresh-cooldown")       ← cooldown timer starts
      emit("refresh-done")           ← spinner stops
  → returns UsageData to JS
```

### Background polling

`start_polling()` in `lib.rs` spawns a Tokio task that loops forever:

1. Sleep for `poll_interval_secs` (from settings)
2. Check `auto_poll` flag — skip if off
3. Check `auth_mode` — skip if not authenticated
4. Call `fetch_claude_usage()`
5. Update `UsageCache`
6. Emit `usage-updated` (both windows refresh)
7. Run notification rule evaluation

### Cache

`UsageCache` is a `Mutex<Option<(UsageData, u64)>>` managed by Tauri:

```rust
pub struct UsageCache(pub Mutex<Option<(UsageData, u64)>>);
//                                       ^          ^
//                                       data     Unix ms timestamp
```

`get_cached_usage` returns `Option<CachedUsage>` with both fields. The tray menu and API server always read from this cache — they never trigger a fresh fetch on their own (unless the user clicks refresh).

---

## Settings system

Settings are stored as a JSON object under the key `"settings"` in `store.json` (a Tauri plugin-store file in the app data directory).

### Rust struct (`commands.rs`)

```rust
pub struct Settings {
    // General
    pub launch_at_startup: bool,
    pub show_in_tray: bool,
    pub minimize_to_tray: bool,

    // Sync
    pub auto_poll: bool,
    pub poll_interval_secs: u64,   // 30 | 60 | 300 | 900
    pub foreground_poll: bool,

    // Notifications (desktop)
    pub notifications_enabled: bool,
    pub notification_rules: Vec<NotificationRule>,

    // ntfy
    pub ntfy_enabled: bool,
    pub ntfy_server: String,
    pub ntfy_topic: String,
    pub ntfy_rules: Vec<NotificationRule>,

    // Display
    pub precise_timestamp: bool,
    pub hide_cooldown_badge: bool,
    pub show_reset_tooltip: bool,

    // API server
    pub api_enabled: bool,
    pub api_port: u16,             // default 7842
    pub api_local_only: bool,      // true = 127.0.0.1, false = 0.0.0.0
    pub api_require_auth: bool,
    pub api_key: String,           // 48-char hex, generated client-side
    pub api_allow_read_usage: bool,
    pub api_allow_refresh: bool,
    pub api_allow_read_settings: bool,
    pub api_allow_write_settings: bool,

    // Debug
    pub debug_devtools: bool,
    pub debug_webview_reload: bool,
}
```

New fields added in future versions must carry `#[serde(default)]` (or `#[serde(default = "fn")]`) so that users upgrading from older versions don't lose their existing settings on the first deserialization.

### TypeScript (`src/lib/types.ts`)

The `Settings` interface mirrors the Rust struct exactly. `DEFAULT_SETTINGS` is used as the initial state before the backend responds and as the fallback if `get_settings` fails.

### Reading settings

```rust
// Rust
let settings: Settings = app
    .store("store.json").ok()
    .and_then(|s| s.get("settings"))
    .and_then(|v| serde_json::from_value(v).ok())
    .unwrap_or_default();
```

```typescript
// TypeScript
const settings = await invoke<Settings>("get_settings");
```

### Saving settings

All writes go through `save_settings`. It applies side-effects synchronously before writing to disk:

1. Toggle autostart via `tauri-plugin-autostart`
2. Show/hide the tray icon via `TrayState`
3. Start/stop/reconfigure the API server via `api::apply()`
4. Write the JSON to `store.json`

The Settings page debounces saves by 300 ms after every state change.

---

## Tauri event bus

Events are broadcast globally (`app.emit(...)`) so all open windows receive them.

| Event | Direction | Payload | Description |
|-------|-----------|---------|-------------|
| `usage-updated` | Rust → JS | `UsageData` | A background poll or tray refresh succeeded |
| `usage-error` | Rust → JS | `string` | A fetch or poll failed |
| `refresh-started` | Rust → JS | `null` | A fetch is in progress — show spinner |
| `refresh-done` | Rust → JS | `null` | The fetch finished (success or failure) |
| `refresh-cooldown` | Rust → JS | `null` | Fetch succeeded — start the 60 s cooldown timer |
| `tray-navigate` | Rust → JS | `{ view: "dashboard" \| "settings" }` | Tray button pressed — switch view |
| `tray-menu-orientation` | Rust → JS | `{ arrow: "up" \| "down" }` | Tray popup repositioned — update arrow direction |

`refresh-started`, `refresh-done`, and `refresh-cooldown` are emitted by every fetch path (user-triggered, tray-triggered, API-triggered) so both windows always stay in sync.

### Listening in React

```typescript
import { listen } from "@tauri-apps/api/event";

useEffect(() => {
  const unlisten = listen<UsageData>("usage-updated", (e) => {
    setUsage(e.payload);
  });
  return () => { unlisten.then((f) => f()); };
}, []);
```

---

## Tray menu

The tray menu is a second `WebviewWindow` labeled `"tray-menu"`. It is:

- Created lazily on first click (not at startup)
- `always_on_top`, `skip_taskbar`, `decorations: false`, `transparent: true`
- Auto-hidden on focus loss after a 300 ms grace period (to avoid spurious WebView2 init events)
- Positioned above or below the tray icon depending on available screen space

### Click debounce

`TrayLastHide` stores the `Instant` when the menu last hid due to focus loss. If the tray icon is clicked within 300 ms of that instant, the click is treated as "close" and the menu is not reopened. This prevents the icon click that dismisses an open menu from immediately reopening it.

### Tray actions (JS → Rust)

```typescript
import { invoke } from "@tauri-apps/api/core";

invoke("tray_action", { action: "show" });      // open main window → dashboard
invoke("tray_action", { action: "settings" });  // open main window → settings
invoke("tray_action", { action: "refresh" });   // trigger background refresh
invoke("tray_action", { action: "quit" });      // exit the process
```

---

## Notification rules

Rules are evaluated after every successful background poll. Each rule is edge-triggered (fires only on the transition, not on every poll).

### Rule types

| Type | Fires when |
|------|------------|
| `threshold` | A window's utilization crosses **above** `at_pct` (rising edge) |
| `spike` | A window's utilization jumps by ≥ `by_pct` since the last poll |
| `reset_soon` | A window's countdown crosses **below** `within_mins` minutes |
| `recovery` | A window's utilization crosses **below** `below_pct` (falling edge) |

All rules are suppressed on the very first poll after startup to avoid a burst of notifications when the app launches into an already-high-usage state.

The `window` field can be `"five_hour"`, `"seven_day"`, `"seven_day_sonnet"`, or `"any"` (threshold and recovery only — evaluates the maximum utilization across all windows).

The same rule format is used for both desktop notifications and ntfy. Each rule set is independent.

---

## HTTP API server

The API server is an optional axum 0.7 HTTP server that runs inside the Tauri process. It is disabled by default. Enable it in **Settings → API**.

### Configuration

All settings are persisted alongside the rest of the app settings.

| Setting | Default | Description |
|---------|---------|-------------|
| `api_enabled` | `false` | Master switch |
| `api_port` | `7842` | TCP port to listen on |
| `api_local_only` | `true` | `true` = bind to `127.0.0.1`, `false` = bind to `0.0.0.0` |
| `api_require_auth` | `true` | Require `Authorization: Bearer <key>` |
| `api_key` | `""` | 48-char hex key, generated in the UI |
| `api_allow_read_usage` | `true` | Enable `GET /usage` |
| `api_allow_refresh` | `false` | Enable `POST /usage/refresh` |
| `api_allow_read_settings` | `false` | Enable `GET /settings` |
| `api_allow_write_settings` | `false` | Enable `PATCH /settings` |

The server starts, stops, and reconfigures itself automatically whenever settings are saved — no restart required.

### Base URL

```
http://127.0.0.1:7842        # local-only (default)
http://0.0.0.0:7842          # network-accessible
```

### Authentication

When `api_require_auth` is enabled, every request must include:

```
Authorization: Bearer <api_key>
```

Requests without a valid token receive `401 Unauthorized`:

```json
{
  "error": "Unauthorized",
  "hint": "Pass your API key as 'Authorization: Bearer <key>'"
}
```

When auth is disabled, the header is ignored entirely.

### Endpoints

---

#### `GET /`

Health check. Always permitted, no permission flag required.

**Response `200`**

```json
{
  "status": "ok",
  "app": "Claudeometer",
  "version": "0.1.0",
  "endpoints": [
    { "method": "GET",   "path": "/usage",         "permission": "read_usage"     },
    { "method": "POST",  "path": "/usage/refresh", "permission": "refresh"        },
    { "method": "GET",   "path": "/settings",      "permission": "read_settings"  },
    { "method": "PATCH", "path": "/settings",      "permission": "write_settings" }
  ]
}
```

**Example**

```bash
curl http://127.0.0.1:7842/
```

---

#### `GET /usage`

Returns the most recently cached usage data. Requires `api_allow_read_usage`.

**Response `200`**

```json
{
  "data": {
    "five_hour": {
      "utilization": 0.42,
      "resets_at": "2026-05-14T19:00:00Z"
    },
    "seven_day": {
      "utilization": 0.17,
      "resets_at": "2026-05-19T00:00:00Z"
    },
    "seven_day_sonnet": null,
    "org_name": "Personal",
    "name": "Antoine Rossi",
    "email": "user@example.com",
    "fetched_at": "2026-05-14T12:34:56Z",
    "source": "claude_ai"
  },
  "fetched_at_ms": 1747223696000
}
```

| Field | Type | Description |
|-------|------|-------------|
| `data.five_hour` | object \| null | 5-hour rolling window |
| `data.seven_day` | object \| null | 7-day rolling window |
| `data.seven_day_sonnet` | object \| null | 7-day Sonnet-specific window |
| `data.*.utilization` | number | 0.0 – 1.0 (not a percentage) |
| `data.*.resets_at` | string \| null | ISO 8601 UTC timestamp |
| `data.fetched_at` | string | ISO 8601 UTC timestamp of the fetch |
| `fetched_at_ms` | number | Unix milliseconds of the fetch |

**Response `503`** — no data in cache yet (app just started, no successful poll)

```json
{ "error": "No data yet — try POST /usage/refresh" }
```

**Response `403`** — permission not enabled

```json
{ "error": "read_usage not enabled in API permissions" }
```

**Example**

```bash
curl http://127.0.0.1:7842/usage \
  -H "Authorization: Bearer a1b2c3d4e5f6..."
```

```python
import requests

resp = requests.get(
    "http://127.0.0.1:7842/usage",
    headers={"Authorization": "Bearer a1b2c3d4e5f6..."},
)
data = resp.json()
five_hour_pct = data["data"]["five_hour"]["utilization"] * 100
print(f"5-hour usage: {five_hour_pct:.0f}%")
```

---

#### `POST /usage/refresh`

Triggers a usage refresh in the background. Requires `api_allow_refresh`.

The request returns immediately — the actual fetch happens asynchronously. Both the main window and tray menu will update when it completes (via the standard `refresh-started` / `refresh-done` events).

**Response `200`**

```json
{ "status": "refresh triggered" }
```

**Response `403`** — permission not enabled

```json
{ "error": "refresh not enabled in API permissions" }
```

**Example**

```bash
curl -X POST http://127.0.0.1:7842/usage/refresh \
  -H "Authorization: Bearer a1b2c3d4e5f6..."
```

---

#### `GET /settings`

Returns the current settings object. Requires `api_allow_read_settings`. The `api_key` field is always omitted from the response.

**Response `200`**

```json
{
  "launch_at_startup": false,
  "show_in_tray": true,
  "minimize_to_tray": true,
  "auto_poll": true,
  "poll_interval_secs": 60,
  "foreground_poll": true,
  "notifications_enabled": true,
  "notification_rules": [],
  "ntfy_enabled": false,
  "ntfy_server": "https://ntfy.sh",
  "ntfy_topic": "claudeometer",
  "ntfy_rules": [],
  "precise_timestamp": false,
  "hide_cooldown_badge": false,
  "show_reset_tooltip": true,
  "api_enabled": true,
  "api_port": 7842,
  "api_local_only": true,
  "api_require_auth": true,
  "api_allow_read_usage": true,
  "api_allow_refresh": false,
  "api_allow_read_settings": true,
  "api_allow_write_settings": false,
  "debug_devtools": false,
  "debug_webview_reload": false
}
```

**Example**

```bash
curl http://127.0.0.1:7842/settings \
  -H "Authorization: Bearer a1b2c3d4e5f6..."
```

---

#### `PATCH /settings`

Partially updates settings. Requires `api_allow_write_settings`.

Only the following fields are patchable via the API. API server configuration, authentication, notification rules, and debug flags cannot be changed remotely.

| Field | Type | Description |
|-------|------|-------------|
| `auto_poll` | boolean | Enable/disable background polling |
| `poll_interval_secs` | number | Polling interval: `30`, `60`, `300`, or `900` |
| `foreground_poll` | boolean | Refresh when the window gains focus |
| `precise_timestamp` | boolean | Show exact time instead of relative |
| `hide_cooldown_badge` | boolean | Hide the countdown on the refresh button |
| `show_reset_tooltip` | boolean | Show reset-time tooltip on usage bars |

Send only the fields you want to change; omitted fields are left unchanged.

**Request body**

```json
{ "auto_poll": false }
```

**Response `200`**

```json
{ "status": "updated" }
```

**Response `422`** — malformed JSON or wrong field type

**Response `403`** — permission not enabled

**Example**

```bash
# Pause polling
curl -X PATCH http://127.0.0.1:7842/settings \
  -H "Authorization: Bearer a1b2c3d4e5f6..." \
  -H "Content-Type: application/json" \
  -d '{"auto_poll": false}'

# Resume polling at 5-minute intervals
curl -X PATCH http://127.0.0.1:7842/settings \
  -H "Authorization: Bearer a1b2c3d4e5f6..." \
  -H "Content-Type: application/json" \
  -d '{"auto_poll": true, "poll_interval_secs": 300}'
```

---

### Error responses

All error responses use the same shape:

```json
{ "error": "human-readable message" }
```

| Status | Meaning |
|--------|---------|
| `401` | Missing or invalid API key |
| `403` | Endpoint exists but the permission flag is disabled |
| `422` | Request body could not be parsed |
| `503` | Cache is empty (no data yet) |
| `500` | Internal error (store or serialization failure) |

### CORS

The server includes permissive CORS headers (`Access-Control-Allow-Origin: *`), so you can call it from a browser or the Tauri settings page's built-in test button without any preflight issues.

---

## Adding new features

### New setting

1. Add the field to `Settings` in `commands.rs` with `#[serde(default)]` and a sensible default in `impl Default for Settings`
2. Add the matching field to the `Settings` interface and `DEFAULT_SETTINGS` in `src/lib/types.ts`
3. Add a control in the appropriate `<Section>` in `src/views/Settings.tsx`
4. If the setting requires a side-effect on save (like the tray or API server), apply it in `save_settings` in `commands.rs`

### New Tauri command

1. Write `pub async fn my_command(app: AppHandle, ...) -> Result<T, String>` in `commands.rs`
2. Register it in the `invoke_handler!` macro in `lib.rs`
3. Call it from JS with `invoke<T>("my_command", { ... })`

### New API endpoint

1. Write a handler `async fn handle_foo(State(state): State<ApiState>) -> Response` in `api.rs`
2. Add the permission field to `Settings` in `commands.rs` and `types.ts`
3. Check the permission at the top of the handler using `load_settings(&state.app)`
4. Register the route in `run_server()` in `api.rs`
5. Add the route to the endpoint list in `handle_root()`

### New event

Emit from Rust: `app.emit("my-event", payload)?;`

Listen in React:
```typescript
import { listen } from "@tauri-apps/api/event";

useEffect(() => {
  const unlisten = listen<MyPayload>("my-event", (e) => {
    // handle e.payload
  });
  return () => { unlisten.then((f) => f()); };
}, []);
```

Because events are broadcast globally, any window that has registered a listener will receive them — useful for keeping the main window and tray menu in sync without explicit targeting.
