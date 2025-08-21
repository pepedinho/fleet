# Fleet

Fleet is a lightweight Rust-based tool for **automated repository monitoring and updating**.
It runs a background daemon (`fleetd`) that watches your Git repositories, detects changes on remote branches, and executes predefined update commands when changes are found.

Its goal is to make **continuous deployment and synchronization** easy without relying on heavy CI/CD pipelines.

---

## Features

* Watch multiple repositories at once
* Automatically detect new commits on remote branches
* Execute update scripts when changes are found
* Per-project logs accessible via CLI
* Start, stop, and resume repository watches dynamically
* YAML-based configuration for flexible update workflows
* Parallel and sequential job execution in pipelines
* Detect cyclic dependencies in pipeline jobs
* Optional per-step environment variables and container execution
* Respect blocking and non-blocking step configuration

---

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/fleet.git
cd fleet

# install fleet
make install
```

---

## Commands

### `fleet watch`

Add a project to the watch list. (you must run this command in the directory of the project to be monitored)

```bash
fleet watch
```

---

### `fleet logs [id | name]`

Show logs for a given project.

* **Without arguments:**
  Shows logs for the project in the current directory (if watched).
* **With `id` or `name`:**
  Shows logs for the specified project.

---

### `fleet ps`

List watched projects.

* Default: only active watches
* `-a`: also show stopped projects

---

### `fleet stop <id>`

Stop watching a project by ID.

---

### `fleet up <id>`

Resume watching a previously stopped project.

---

### `fleet rm <id>`

Remove a monitored project from the watch list.

---

## YAML Configuration

Each project has a `fleet.yml` file defining its pipelines process.

### Example `.fleet.yml`

```yaml
timeout: 200 # Timeout in seconds for non-blocking commands (default 300)

pipeline:
  jobs:
    build:
      steps:
        - cmd: cargo build

    test_rust:
      needs: [build]
      env:
        RUST_LOG: debug
      steps:
        - cmd: cargo test

    echo_test:
      needs: [build]
      steps:
        - cmd: echo test 1
          container: ubuntu:latest

    deploy:
      needs: [test_rust, echo_test]
      steps:
        - cmd: echo je suis arriver a la fin
          blocking: true
```

### Key Points

* **timeout**: Global timeout for non-blocking commands.
* **pipeline**: Define jobs, their dependencies (`needs`), and execution steps.

  * `blocking: true` → fire and forget.
  * `blocking: false` (default) → Run asynchronously with timeout.
  * `env` → Optional environment variables for steps.
  * `container` → Optional Docker container to execute step.
* **on\_conflict**: Commands to run when a Git conflict is detected.
* **post\_update**: Commands to run after updates complete.

---

## How It Works

1. `fleetd` runs in the background, periodically checking for updates.
2. When a new commit is found:

   * The pipeline jobs are executed respecting dependencies and blocking configuration.
   * Jobs run in parallel when independent.
   * If a job fails, dependent jobs are not executed and the failure is propagated.
   * Cyclic dependencies are detected at pipeline start and reported as errors.
   * Step-specific environment variables and container execution are supported.
   * If a conflict occurs, `on_conflict` commands are executed.
   * Finally, `post_update` commands run.
3. Logs for each project are stored and retrievable via `fleet logs`.
