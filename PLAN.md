# OxiTrack MVP - Minimal Viable Plan

## Core Philosophy

Ship something you can use Monday morning. One weekend of work, no background tasks, no separate services, no stats UI. If you can't validate the idea with this, more code won't help.

## Tech Stack

- **Framework**: Axum (just for the POST endpoint)
- **Database**: SQLite with SQLx (single table)
- **Everything else**: Skip it. No tracing, no config file, no migrations directory.

## Database Schema

One table. That's it.

```sql
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    project_path TEXT NOT NULL,
    start_time INTEGER NOT NULL,  -- Unix timestamp
    last_heartbeat INTEGER NOT NULL,
    UNIQUE(project_path, start_time) ON CONFLICT REPLACE
)
```

No separate projects table. No heartbeats table. `last_heartbeat` gets updated on every beat. When you query, subtract `start_time` from `MAX(last_heartbeat)`.

## API Design

**Single endpoint: `POST http://localhost:3000/beat`**

Request body:

```json
{
  "project_path": "/home/you/code/oxitrack",
  "timestamp": 1735219200
}
```

Response:

```json
{
  "session_id": 123,
  "project_path": "/home/you/code/oxitrack",
  "duration_seconds": 847
}
```

No GET endpoints yet. Query the SQLite file directly with `sqlite3` to verify: `SELECT project_path, MAX(last_heartbeat) - start_time FROM sessions GROUP BY project_path;`

## Rust Daemon Structure

```
src/
├── main.rs          # 100 lines: setup Axum, one handler, one DB query
└── db.rs            # 50 lines: init_db() and update_session()
```

**main.rs** does three things:

1. Open SQLite connection (create file if missing)
2. Run `CREATE TABLE IF NOT EXISTS sessions ...`
3. Start Axum server with one route: `post("/beat", handle_heartbeat)`

**handle_heartbeat** logic:

```rust
// 1. Canonicalize project_path (resolve symlinks)
// 2. Query: SELECT id FROM sessions
//    WHERE project_path = ? AND last_heartbeat > ? - 900
//    ORDER BY last_heartbeat DESC LIMIT 1
// 3. If found: UPDATE sessions SET last_heartbeat = ? WHERE id = ?
// 4. If not found: INSERT INTO sessions (project_path, start_time, last_heartbeat) VALUES (?, ?, ?)
// 5. Return session_id and calculated duration
```

No background task. Sessions "close" implicitly when you query them. If `last_heartbeat` is older than 5 minutes, it's not active.

## VSCode Plugin (50 lines)

`extension.ts`:

```typescript
import * as vscode from "vscode";

let lastProject: string | null = null;

function sendHeartbeat(projectPath: string) {
  fetch("http://localhost:3000/beat", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      project_path: projectPath,
      timestamp: Math.floor(Date.now() / 1000),
    }),
  }).catch(() => {}); // Fail silently
}

export function activate(context: vscode.ExtensionContext) {
  // Send heartbeat every 60 seconds
  setInterval(() => {
    const workspace = vscode.workspace.workspaceFolders?.[0];
    if (workspace) {
      sendHeartbeat(workspace.uri.fsPath);
    }
  }, 60000);

  // Send heartbeat immediately on file change
  vscode.workspace.onDidChangeTextDocument(() => {
    const workspace = vscode.workspace.workspaceFolders?.[0];
    if (workspace) {
      sendHeartbeat(workspace.uri.fsPath);
    }
  });
}
```

No configuration. No error popups. If the daemon isn't running, the fetch fails silently. Add logging to a file if you need to debug.

## Build & Run

**Daemon:**

```bash
cargo init oxitrack --name oxi
cd oxitrack
# Add axum, sqlx, tokio to Cargo.toml
cargo run
```

**VSCode Plugin:**

```bash
npm install -g yo generator-code
yo code  # Choose TypeScript, fill in details
# Replace extension.ts with the 50 lines above
# F5 to launch Extension Development Host
```

## Validation Plan

1. **Saturday afternoon**: Daemon running, send heartbeat with `curl`. Verify row appears in SQLite.
2. **Saturday evening**: VSCode plugin installed. Open a project, type for 3 minutes. Check SQLite shows ~180 seconds duration.
3. **Sunday morning**: Work on a real task for 30 minutes. Query SQLite. See one row with ~1800 seconds.
4. **Sunday afternoon**: Stop daemon, keep typing in VSCode. Restart daemon. Verify it picks up where it left off (new session created because gap > 15 min).

## Next Steps (After MVP)

Only add features after you've used it for a week and know what hurts:

- **Pain**: Daemon crashes, lose heartbeats → Add simple file buffer in plugin
- **Pain**: Can't remember what I worked on → Add `GET /today` endpoint that returns plain text
- **Pain**: Project path changed → Add project ID hashing
- **Pain**: Want to see trends → Build a tiny web UI with one chart
- **Pain**: Zed extension needed → Research Zed's current extension capabilities (they're limited)

Start here. If this feels too minimal, add one thing. But try this first.
