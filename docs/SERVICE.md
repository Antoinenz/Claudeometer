# Claudeometer headless service

`claudeometer-service` is the GUI-free half of this repo: a small standalone
binary (`crates/service`) that polls claude.ai for your usage limits and
serves the latest reading over a tiny local HTTP API. No tray icon, no
webview, no desktop session required — it's meant to be left running on a
server (a VPS, a home box, a Raspberry Pi) so a coding agent can `curl` it
before it starts new work and stop cleanly instead of getting cut off
mid-task, or so you can glance at your usage from a terminal.

It shares its claude.ai client with the desktop app via the `claudeometer-core`
crate (`crates/core`), so both stay in sync automatically.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/Antoinenz/Claudeometer/main/scripts/install.sh | sh
claudeometer-service login <your-session-key>   # from claude.ai's sessionKey cookie
claudeometer-service install                    # run it in the background from now on
```

Three commands total. If you're comfortable putting the key in an env var for
a fully non-interactive install (e.g. provisioning a server via a script):

```bash
CLAUDEOMETER_SESSION_KEY=<your-session-key> \
  curl -fsSL https://raw.githubusercontent.com/Antoinenz/Claudeometer/main/scripts/install.sh | sh
```

does the download, sign-in, and service registration in one shot.

**Windows**: there's no install script — download
`claudeometer-service-windows-x86_64.exe` from the
[Releases](https://github.com/Antoinenz/Claudeometer/releases) page, then run
`login` and `install` the same way.

**Getting a session key**: open claude.ai in a browser, sign in, open DevTools
→ Application → Cookies → copy the value of `sessionKey`. Same key the
desktop app's Setup step uses.

### What `install` actually does

`install` self-registers the binary as a background service for whatever OS
it's running on — no separate unit files or admin rights to hand-manage:

| Platform | Mechanism | Where |
|---|---|---|
| Linux | `systemd --user` unit, `enable --now` | `~/.config/systemd/user/claudeometer.service` |
| macOS | LaunchAgent, `launchctl bootstrap` | `~/Library/LaunchAgents/com.claudeometer.service.plist` |
| Windows | a real Windows Service (via the Service Control Manager, so it runs with nobody logged in) | registered under the name `Claudeometer` |

On a genuinely headless Linux box (no interactive login ever), also run
`loginctl enable-linger $USER` (printed as a reminder after `install`) so the
systemd user instance — and therefore the service — keeps running after your
SSH session ends.

`uninstall` reverses all of the above. `start`/`stop` control the
already-installed service without removing it.

## CLI reference

```
claudeometer-service login <session-key>   # validate + store credentials
claudeometer-service logout                # remove stored credentials
claudeometer-service status [--json]       # quick glance: reads a running
                                            # service if there is one, else
                                            # does a one-off direct fetch
claudeometer-service run [--bind ADDR] [--interval SECS]
                                            # foreground: poll + serve, blocks
                                            # (this is what installed services exec)
claudeometer-service install / uninstall   # register/remove the OS service
claudeometer-service start / stop          # control the installed service
claudeometer-service config [--bind ADDR] [--interval SECS]
                            [--api-key KEY | --generate-api-key | --clear-api-key]
                                            # view/edit the config file
                                            # instead of hand-editing JSON
```

`status` is the "quick glance" entry point:

```bash
$ claudeometer-service status
Claude usage — Pro tier (running service)
  5-hour           [########............]   42%  resets in 3h 12m
  7-day            [###.................]   17%  resets in 4d 2h
  7-day (Sonnet)   (not available)
```

## Config file

`~/.config/claudeometer/config.json` (Linux/macOS) or
`%APPDATA%\claudeometer\config.json` (Windows):

```json
{
  "bind": "127.0.0.1:7842",
  "poll_interval_secs": 60,
  "api_key": null
}
```

| Field | Default | Notes |
|---|---|---|
| `bind` | `127.0.0.1:7842` | Set to `0.0.0.0:7842` to make it reachable from other machines (e.g. an agent running elsewhere) — do this via `claudeometer-service config --bind 0.0.0.0:7842 --generate-api-key`, not by hand-editing, so you don't forget the key. |
| `poll_interval_secs` | `60` | How often to refetch from claude.ai in the background. |
| `api_key` | `null` | If set, every request needs `Authorization: Bearer <key>`. The service refuses to bind to a non-loopback address without one (prints a loud warning and runs anyway rather than hard-failing, so you notice on the very first log line). |

Credentials (the claude.ai session key) are **not** in this file — they go
through the OS keyring (Keychain / Credential Manager / libsecret) when one is
available, or a `chmod 600` file (`~/.config/claudeometer/session_key`) when
it isn't, which is the common case on a bare Linux server with no desktop
session or dbus. `login`/`logout` manage this for you either way. A machine
that already has the desktop app signed in needs no separate `login` step —
they share the same keyring entry.

## HTTP API

Smaller than the desktop app's embedded API on purpose: this process is meant
to sit on a server, possibly network-reachable, so there's no remote settings
mutation here — only reading usage and triggering a refresh.

### `GET /`

Health check, no auth required regardless of `api_key`.

```json
{ "status": "ok", "app": "claudeometer-service", "version": "0.1.0", "endpoints": [...] }
```

### `GET /usage`

The endpoint an agent curls before doing more work. Same response shape as
the desktop app's `/usage`, so anything already scripted against that keeps
working:

```bash
curl http://127.0.0.1:7842/usage
```

```json
{
  "data": {
    "five_hour": { "utilization": 0.42, "resets_at": "2026-07-02T19:00:00Z" },
    "seven_day": { "utilization": 0.17, "resets_at": "2026-07-06T00:00:00Z" },
    "seven_day_sonnet": null,
    "org_name": "Personal",
    "name": "Antoine Rossi",
    "email": "user@example.com",
    "tier": "Pro",
    "fetched_at": "2026-07-02T12:34:56Z",
    "source": "claude_ai"
  },
  "fetched_at_ms": 1751452800000
}
```

`utilization` is `0.0`–`1.0` (multiply by 100 for a percentage). `503` means
no successful poll has happened yet — `POST /usage/refresh` to kick one off.

A minimal agent-side stop check:

```bash
pct=$(curl -sf http://127.0.0.1:7842/usage | python3 -c \
  'import sys,json; print(json.load(sys.stdin)["data"]["five_hour"]["utilization"]*100)')
if awk "BEGIN{exit !($pct >= 90)}"; then
  echo "5-hour usage at ${pct}% — stopping cleanly before I get cut off."
  exit 0
fi
```

### `POST /usage/refresh`

Triggers an immediate background refetch; returns right away.

```bash
curl -X POST http://127.0.0.1:7842/usage/refresh
```

### Auth

If `api_key` is set in the config file, every request (except `GET /`) needs:

```
Authorization: Bearer <api_key>
```

Missing/invalid → `401`. `curl -H "Authorization: Bearer <key>" ...`.

## Logs

`run` logs each poll (success or failure) to stdout/stderr with a timestamp.
Under an installed service:

- Linux: `journalctl --user -u claudeometer -f`
- macOS: `tail -f /tmp/claudeometer-service.log`
- Windows: Event Viewer (or run `claudeometer-service run` in a console directly to watch it live)

## Relationship to the desktop app

Both binaries link the same `claudeometer-core` crate for talking to
claude.ai (`crates/core/src/claude.rs`) — no duplicated HTTP client to drift
out of sync. They intentionally have separate, independently-sized settings
and API surfaces: the desktop app's is tray/notification/UI-oriented
(`docs/DEVELOPER.md`), the service's is deliberately minimal for running
unattended on a server.
