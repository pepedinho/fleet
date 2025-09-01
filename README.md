<h1>
  <img src="https://github.com/user-attachments/assets/30281fa1-3b9e-4ba9-a865-050467f2a509" alt="brain" width="50" height="50"/>
  Fleet
</h1>

![Rust](https://img.shields.io/badge/rust-stable-orange)

Fleet is a lightweight Rust-based tool for **automated repository monitoring and updating**.
It runs a background daemon (`fleetd`) that watches your Git repositories, detects remote changes, and executes predefined update commands.

Its goal is to make **continuous deployment and synchronization** simple without relying on heavy CI/CD pipelines.

---

<h2>
  <img src="https://github.com/user-attachments/assets/4bf0a9f7-f5b7-4401-9b3d-fc92523cb79c" alt="brain" width="30" height="30"/>
  Summary
</h2>

* [Features](#features)
* [Quick Start](#quick-start)
* [Commands](#commands)
* [Configuration](#configuration)
* [How It Works](#how-it-works)

---

<h2 id="features">
  <img src="https://github.com/user-attachments/assets/dc7fc109-abb2-443a-9bc3-8f6721cdd1e8" alt="brain" width="40" height="40"/>
  Features
</h2>

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
* Notifications on pipeline completion (only discord for now)
* Statistics overview of watched projects with CPU/memory usage and success/failure counts

---

<h2 id="quick-start">
  <img src="https://github.com/user-attachments/assets/e0fdb113-496a-4d47-91a5-008166f355a8" alt="brain" width="40" height="25"/>
  Quick Start
</h2>


```bash
# Clone the reposito![Wikipedia_iOS_Sticker_-_Idea_2](https://github.com/user-attachments/assets/7ea987db-7193-461b-a715-9ecf02e8c76c)
ry
git clone https://github.com/pepedinho/fleet.git
cd fleet

# Install fleet
make install
```

Add your first project:

```bash
fleet watch
```

---


<h2 id="commands">
  <img src="https://github.com/user-attachments/assets/4444209c-0c59-4757-aad1-b0956226d7b9" alt="brain" width="30" height="30"/>
  Commands
</h2>

| Command                 | Description                                                                            |
| ----------------------- | -----------------------------------------------------------------------------          |
| `fleet watch`           | Add a project to the watch list (run inside the project dir) (`-b` to assign branch)   |
| `fleet logs [id\|name]` | Show logs for a project (current dir by default) (`-f` to follow logs)                 |
| `fleet ps`              | List watched projects (`-a` to show stopped projects)                                  |
| `fleet stop <id>`       | Stop watching a project                                                                |
| `fleet up <id>`         | Resume watching a stopped project                                                      |
| `fleet rm <id>`         | Remove a monitored project                                                             |
| `fleet stats`           | Show interactive statistics of all watched projects                                    |

---


<h2 id="configuration">
  <img src="https://github.com/user-attachments/assets/47ba484c-3bec-43d4-8b50-1e03456709c2" alt="brain" width="30" height="30"/>
  Configuration
</h2>

Each project defines its pipelines with a `fleet.yml` file.

<details>
<summary>Example fleet.yml</summary>

```yaml
timeout: 200 # Timeout in seconds for non-blocking commands (default 300)

pipeline:
  notifications:
    on: [success, failure]
    channels:
      - type: discord
        url: https://discord.com/api/webhooks/...

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
        - cmd: echo "deploy complete"
          blocking: true
```

</details>

**Key Points:**

* `timeout` → global timeout for async jobs (default 300s).
* `needs` → define dependencies between jobs.
* `blocking: true` → fire and forget.
* `env` → per-step environment variables.
* `container` → run step in Docker container.
* `notifications` → external alerts (success/failure).

---
<h2 id="how-it-works">
  <img src="https://github.com/user-attachments/assets/a18b44ad-ff8b-4d7f-a7ae-5fafa8d19449" alt="brain" width="30" height="30"/>
  How It Works
</h2>

<details>
<summary>Detailed workflow</summary>

1. `fleetd` runs in the background and periodically checks repositories.
2. When a new commit is detected:

   * Jobs are executed respecting dependencies.
   * Independent jobs run in parallel.
   * Failures propagate and block dependent jobs.
   * Cyclic dependencies are detected and reported before execution.
   * Environment variables and containers are supported per step.
   * `on_conflict` and `post_update` hooks can be executed.
   * Notifications are sent to configured channels (Discord, webhook, etc.).
3. Logs for each project are stored and retrievable via `fleet logs`.
4. Global statistics are available via `fleet stats`.

</details>

---
