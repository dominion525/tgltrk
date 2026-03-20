# tgltrk

[![CI](https://github.com/dominion525/tgltrk/actions/workflows/ci.yml/badge.svg)](https://github.com/dominion525/tgltrk/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)
[![MSRV: 1.85](https://img.shields.io/badge/MSRV-1.85-orange)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)

[日本語版 (Japanese)](README.ja.md)

Unofficial Toggl Track CLI — a thin wrapper over the Toggl Track API v9 for managing time tracking from the command line.

## Installation

### GitHub Releases

Download a prebuilt binary for your platform from [Releases](https://github.com/dominion525/tgltrk/releases).

Supported platforms:
- Linux (x86_64)
- macOS (Apple Silicon / Intel)
- Windows (x86_64)

### Build from source

```bash
cargo install --path .
```

Requires Rust 1.85.0 or later.

## Setup

Set your Toggl Track API token. You can find it in [Profile Settings](https://track.toggl.com/profile).

```bash
# Interactive input (recommended - not saved in shell history)
tgltrk auth login

# Or use an environment variable
export TOGGL_API_TOKEN=your_token_here
```

Check authentication status:

```bash
$ tgltrk auth status
✓ Authenticated as Alice (alice@example.com)
```

## Usage

### Timer

```bash
# Show current timer
$ tgltrk timer current
#12345678 Code review 01:23:45 [running]

# Start a timer
$ tgltrk timer start -d "Code review" -p 12345 -t bug,review
✓ Timer started
#12345679 Code review 00:00:00 [running] [bug, review]

# Stop the timer
$ tgltrk timer stop
✓ Timer stopped
#12345679 Code review 00:45:12
```

### Time entries

```bash
# List recent entries
$ tgltrk entries list -n 3
#12345677 Weekly meeting 01:00:00
#12345676 Bug fix 00:30:15
#12345675 Design review 02:15:00 [design]

# Filter by date range
$ tgltrk entries list --since 2024-01-01 --until 2024-01-31

# Get entry details
$ tgltrk entries get 12345677
#12345677 Weekly meeting 01:00:00

# Create an entry with specific times
$ tgltrk entries create --start "2024-01-15 09:00" --stop "2024-01-15 10:30" -d "Morning meeting"
✓ Entry created
#12345678 Morning meeting 01:30:00

# Create with duration instead of stop time
$ tgltrk entries create --start "2024-01-15 14:00" --duration "2h" -d "Coding session"
✓ Entry created
#12345679 Coding session 02:00:00

# Edit an entry
$ tgltrk entries edit 12345677 -d "Standup meeting" -b true
✓ Entry updated
#12345677 Standup meeting 01:00:00

# Edit start/stop time
$ tgltrk entries edit 12345677 --start "2024-01-15 09:30" --duration "45m"
✓ Entry updated
#12345677 Standup meeting 00:45:00

# Delete an entry
$ tgltrk entries delete 12345677
✓ Entry #12345677 deleted

# Continue a previous entry (starts a new timer with the same settings)
$ tgltrk entries continue 12345676
✓ Timer continued
#12345680 Bug fix 00:00:00 [running]
```

### Projects

```bash
$ tgltrk projects list
#1001 Website Redesign
#1002 Mobile App
#1003 API Migration (archived)

$ tgltrk projects create "New Project"
✓ Project created
#1004 New Project

$ tgltrk projects update 1004 --name "Renamed Project"
✓ Project updated
#1004 Renamed Project

$ tgltrk projects delete 1004
✓ Project #1004 deleted
```

### Tags

```bash
$ tgltrk tags list
#501 bug
#502 review
#503 design

$ tgltrk tags create "urgent"
✓ Tag created
#504 urgent

$ tgltrk tags delete 504
✓ Tag #504 deleted
```

### Cache

User info, projects, and tag lists are cached for 72 hours to reduce API calls. Toggl Track API has rate limits, and caching helps stay within those limits during normal usage.

```bash
$ tgltrk cache status
Cache dir: /Users/alice/Library/Caches/com.tgltrk.tgltrk
  user: 245 bytes, last updated 2024-01-15 10:30:00 UTC
  projects_1234: 1024 bytes, last updated 2024-01-15 10:30:01 UTC

$ tgltrk cache clear
✓ Cache cleared
```

Cache is automatically invalidated when you create, update, or delete projects/tags. Switching accounts via `auth login` clears all cached data.

## Global options

```
--json         Output in JSON format
--workspace    Override workspace ID
```

JSON output includes cache hit metadata:

```json
{
  "meta": { "cached": ["user"] },
  "data": {
    "email": "alice@example.com",
    "fullname": "Alice",
    "default_workspace_id": 1234,
    "timezone": "Asia/Tokyo"
  }
}
```

## Credential storage

API tokens are stored in the OS native keyring (macOS Keychain / Windows Credential Manager / Linux Secret Service). If the keyring is unavailable, use the `TOGGL_API_TOKEN` environment variable instead.

## Limitations

- **No bulk operations**: batch delete or batch edit of multiple entries is not supported
- **No reporting**: Toggl Track's reporting endpoints (Summary, Detailed, Weekly) are not supported
- **No workspace management**: workspaces cannot be created or modified; `--workspace` only selects an existing one
- **Paid features**: features exclusive to paid Toggl Track plans (e.g., project templates, time estimates, required fields) are not supported
- **Rate limits**: Toggl Track API enforces rate limits. The CLI caches user info, projects, and tags for 72 hours to minimize API calls. If you hit rate limits, wait for the rate limit to reset before retrying

## License

MIT OR Apache-2.0
