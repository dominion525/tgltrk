---
name: tgltrk
description: >
  Operate Toggl Track time tracking from the CLI. Start/stop timers,
  list/edit/delete/continue time entries, manage projects and tags.
  Use when the user mentions Toggl Track, time tracking, timers, or work logging.
---

## Prerequisites

- API token must be configured (`tgltrk auth login` or environment variable `TOGGL_API_TOKEN`)

## Workflow

### Starting a timer

When the user asks to track work:

1. **Determine if enough information is provided**
   - User specifies project name, tags, and description explicitly → run `timer start` directly
   - Ambiguous request (e.g., "track this work") → proceed to next step

2. **Fetch project and tag candidates**
   ```
   tgltrk projects list
   tgltrk tags list
   ```

3. **Suggest the best match and confirm**
   - Infer the most appropriate project and tags from the work context
   - Ask the user: "Start with project: X, tags: Y?"
   - If no match is found, present the list and let the user choose

4. **Execute once confirmed**
   ```
   tgltrk timer start -d "description" -p PROJECT_ID -t tag1,tag2
   ```

### Stopping a timer

```
tgltrk timer stop
```

Errors if no timer is running.

### Checking the current timer

```
tgltrk timer current
```

### Continuing a previous entry

Starts a new timer with the same project, tags, and description:

```
tgltrk entries list -n 5
tgltrk entries continue ENTRY_ID
```

### Creating a past entry

Create a completed entry with specific start/stop times:

```
tgltrk entries create --start "2024-01-15 09:00" --stop "2024-01-15 10:30" -d "description" -p PROJECT_ID -t tag1,tag2
```

Use `--duration` instead of `--stop` (e.g. `--duration "1h30m"`, `--duration "90m"`).

### Editing an entry

Only specified fields are updated. All options are optional.

```
tgltrk entries edit ENTRY_ID [-d "description"] [-p PROJECT_ID] [-t tag1,tag2] [-b true|false] [--start "..."] [--stop "..."] [--duration "..."]
```

## Quick Reference

| Purpose                    | Command                                               |
|----------------------------|-------------------------------------------------------|
| Check current timer        | `tgltrk timer current`                                |
| Start timer                | `tgltrk timer start -d "desc" -p PROJECT_ID`          |
| Stop timer                 | `tgltrk timer stop`                                   |
| List entries               | `tgltrk entries list -n 10`                            |
| Filter by date             | `tgltrk entries list --since 2024-01-01 --until 2024-01-31` |
| Create past entry          | `tgltrk entries create --start "..." --stop "..." -d "desc"` |
| Create with duration       | `tgltrk entries create --start "..." --duration "1h30m" -d "desc"` |
| Continue entry             | `tgltrk entries continue ENTRY_ID`                     |
| Edit entry                 | `tgltrk entries edit ENTRY_ID -d "new desc"`           |
| Edit entry time            | `tgltrk entries edit ENTRY_ID --start "..." --stop "..."` |
| Delete entry               | `tgltrk entries delete ENTRY_ID`                       |
| List projects              | `tgltrk projects list`                                 |
| Create project             | `tgltrk projects create "name"`                        |
| List clients               | `tgltrk clients list`                                  |
| Create client              | `tgltrk clients create "name"`                         |
| Update client              | `tgltrk clients update CLIENT_ID --name "new name"`    |
| Delete client              | `tgltrk clients delete CLIENT_ID`                      |
| List tags                  | `tgltrk tags list`                                     |
| Create tag                 | `tgltrk tags create "name"`                            |
| Cache status               | `tgltrk cache status`                                  |
| Clear cache                | `tgltrk cache clear`                                   |

Full options for each command are available via `tgltrk <command> --help`.

## Global Options

- `--json` — Output in JSON format (envelope with metadata). Use for programmatic processing. Plain text is more token-efficient for simple checks
- `--workspace <ID>` — Override default workspace ID

## JSON Output

```json
{
  "meta": { "cached": ["user"] },
  "data": { ... }
}
```

- `meta.cached` — Entities retrieved from cache (empty array = fetched from API)
- `data` — Command result. `null` for delete operations

## Cache Behavior

- User info, projects, and tags are cached for 72 hours (to reduce API calls and stay within rate limits)
- Automatically invalidated on project/tag create, update, or delete
- Cleared entirely on account switch via `auth login`
- Manual clear: `tgltrk cache clear`

## Error Behavior

Errors are printed to stderr as `Error: ...` with exit code 1. Triggered by missing auth, API errors, invalid date format, etc.

## Limitations

- Workspaces cannot be created or modified (`--workspace` selects only)
- Reporting endpoints (Summary, Detailed, Weekly) are not supported
- Bulk operations (e.g., batch delete) are not supported
