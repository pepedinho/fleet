# Fleet

Fleet is a lightweight Rust-based tool for **automated repository monitoring and updating**.  
It runs a background daemon (`fleetd`) that watches your Git repositories, detects changes on remote branches, and executes predefined update commands when changes are found.  

Its goal is to make **continuous deployment and synchronization** easy without relying on heavy CI/CD pipelines.

---

## Features

- Watch multiple repositories at once
- Automatically detect new commits on remote branches
- Execute update scripts when changes are found
- Per-project logs accessible via CLI
- Start, stop, and resume repository watches dynamically
- YAML-based configuration for flexible update workflows

---

## Installation

> **Note:** Installation scripts and daemonization setup are not yet included.  
For now, compile and run manually.

```bash
# Clone the repository
git clone https://github.com/yourusername/fleet.git
cd fleet

# Build in release mode
cargo build --release

# Run the daemon manually
./target/release/fleetd &

# Use the CLI
./target/release/fleet <command>
```

---

## Commands

### `fleet watch`
Add a project to the watch list. (
you must run this command in the directory of the project to be monitored
)

```bash
fleet watch
```

---

### `fleet logs [id | name]`
Show logs for a given project.

- **Without arguments:**  
  Shows logs for the project in the current directory (if watched).
- **With `id` or `name`:**  
  Shows logs for the specified project.

---

### `fleet ps`
List watched projects.

- Default: only active watches
- `-a`: also show stopped projects

---

### `fleet stop <id>`
Stop watching a project by ID.

---

### `fleet up <id>`
Resume watching a previously stopped project.

---

### `fleet rm <id>`
removing a monitored project from the watch list.

---

## YAML Configuration

Each project has a `fleet.yml` file defining its update process and optional conflict or post-update actions.

### Example `.fleet.yml`

```yaml
timeout: 200 # Timeout in seconds for non-blocking commands (default 300)

update:
  - cmd: echo "run update"
  - cmd: ping google.com
    blocking: true

on_conflict:
  - echo "A conflict occurred"
  - git reset --hard origin/main

post_update:
  - echo "Update completed"
```

### Key Points

- **timeout**: Global timeout for non-blocking commands.
- **update**: Commands to run when a new commit is detected.
  - `blocking: true` → fire and forget.
  - `blocking: false` (default) → Run asynchronously with timeout.
- **on_conflict**: Commands to run when a Git conflict is detected.
- **post_update**: Commands to run after updates complete.

---

## How It Works

1. `fleetd` runs in the background, periodically checking for updates.
2. When a new commit is found:
   - The `update` commands are executed (in blocking or non-blocking mode).
   - If a conflict occurs, `on_conflict` commands are executed.
   - Finally, `post_update` commands run.
3. Logs for each project are stored and retrievable via `fleet logs`.
