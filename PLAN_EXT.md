# OxiTrack VSCode Extension - Minimal Plan

## Philosophy

One file, one responsibility: send a heartbeat to localhost:3000 when you're coding. No settings UI, no status bar, no configuration. If the daemon isn't running, it fails silently. You'll know it's working when you see data in SQLite.

## Project Structure

```
oxitrack-vscode/
├── src/
│   └── extension.ts     # Everything lives here
├── package.json         # Bare minimum manifest
└── tsconfig.json        # Standard VSCode extension config
```

## Core Implementation

**src/extension.ts** (80 lines total)

```typescript
import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";

const DAEMON_URL = "http://localhost:3000";
const HEARTBEAT_INTERVAL = 120000; // 2 minutes
const THROTTLE_GAP = 30000; // 30 seconds

let lastBeat = 0;
let timer: NodeJS.Timer | null = null;

// Buffer file for offline heartbeats
const bufferPath = path.join(
  process.env.HOME || process.env.USERPROFILE || "",
  ".oxitrack",
  "buffer.jsonl",
);

async function beat(project: string) {
  const now = Date.now();
  if (now - lastBeat < THROTTLE_GAP) return;

  lastBeat = now;
  const payload = { project_path: project, timestamp: Math.floor(now / 1000) };

  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5000);

    await fetch(`${DAEMON_URL}/beat`, {
      method: "POST",
      body: JSON.stringify(payload),
      headers: { "Content-Type": "application/json" },
      signal: controller.signal,
    });

    clearTimeout(timeout);
  } catch (err) {
    // Write to buffer for manual recovery
    try {
      fs.mkdirSync(path.dirname(bufferPath), { recursive: true });
      fs.appendFileSync(bufferPath, JSON.stringify(payload) + "\n");
    } catch (e) {
      // If we can't even write to buffer, give up silently
    }
  }
}

export function activate(context: vscode.ExtensionContext) {
  const workspace = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!workspace) return;

  // Send heartbeat on file changes (typing)
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument(() => beat(workspace)),
  );

  // Send heartbeat on file switch
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(() => beat(workspace)),
  );

  // Periodic heartbeat while editor is focused
  timer = setInterval(() => {
    if (vscode.window.state.focused) beat(workspace);
  }, HEARTBEAT_INTERVAL);

  // Send initial heartbeat
  beat(workspace);
}

export function deactivate() {
  if (timer) clearInterval(timer);
}
```

**package.json**

```json
{
  "name": "oxitrack",
  "displayName": "OxiTrack",
  "description": "Send heartbeats to OxiTrack daemon",
  "version": "0.0.1",
  "engines": { "vscode": "^1.80.0" },
  "activationEvents": ["*"],
  "main": "./out/extension.js",
  "scripts": {
    "compile": "tsc -p ./",
    "watch": "tsc -watch -p ./"
  },
  "devDependencies": {
    "@types/vscode": "^1.80.0",
    "@types/node": "^20.0.0",
    "typescript": "^5.0.0"
  }
}
```

**tsconfig.json** (standard VSCode extension template)

## Setup Steps

1. **Initialize project**

   ```bash
   mkdir oxitrack-vscode && cd oxitrack-vscode
   npm init -y
   # Copy the package.json content above
   npm install
   ```

2. **Create source file**

   ```bash
   mkdir src
   # Copy extension.ts into src/
   ```

3. **Compile**

   ```bash
   npm run compile
   ```

4. **Test in VSCode**
   - Press F5 to launch Extension Development Host
   - Open a project folder
   - Start typing
   - Check daemon logs: you should see POST requests

## Validation

**Manual test flow:**

1. Terminal 1: `cargo run` (daemon)
2. VSCode: F5 (launch extension host)
3. Open `~/code/test-project`
4. Type randomly for 2 minutes
5. Terminal 1: Should show 3-4 heartbeats
6. Terminal 2: `sqlite3 oxitrack.db "SELECT * FROM sessions"`
7. Should see one row with duration ~120 seconds

**Edge cases to verify:**

- Daemon not running: Type for 1 minute, then start daemon. Check `~/.oxitrack/buffer.jsonl` exists.
- Switch files: Should not send heartbeat within 30s throttle window
- Editor loses focus: Periodic timer stops, resumes on focus

## What You're Not Building (Yet)

- **Configuration UI**: Edit `src/extension.ts` and recompile if you need to change the daemon URL
- **Status bar**: You'll know it's working when you see data
- **Git branch detection**: The daemon doesn't store this yet
- **File-specific tracking**: Only project-level for MVP
- **Multiple workspaces**: Uses first workspace folder only

## Next Steps After MVP

Use it for a week. Then add one thing that actually annoyed you:

- If you keep forgetting to start the daemon → Add a status bar indicator that shows red when daemon is unreachable
- If you work on multiple projects → Send `file_path` and update daemon to handle per-file sessions
- If you want to exclude `node_modules` → Add a simple string match in `beat()` function
- If you switch daemons → Add a configuration entry in package.json and read it with `vscode.workspace.getConfiguration()`

Start with this. If it feels too simple, that's the point.
