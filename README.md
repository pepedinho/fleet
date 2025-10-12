<h1>
  <img src="https://github.com/user-attachments/assets/429bf6e8-5724-473e-a560-e9e06bbbc143" alt="brain" width="30" height="30"/>
  Fleet
</h1>

![Rust](https://img.shields.io/badge/rust-stable-orange)
[![Documentation](https://docs.rs/tokio/badge.svg)](https://pepedinho.github.io/fleet/core_lib/all.html)

Fleet is a **lightweight CI/CD orchestrator** written in Rust.  
Unlike traditional CI/CD systems (GitHub Actions, GitLab CI, Jenkins…), Fleet is designed to run **directly on your host machine** (Raspberry Pi, server, VPS, NAS…).  

It continuously watches your repositories, detects changes, and runs your deployment pipelines **locally**, without relying on external cloud services or heavy infrastructure.  

Think of it as a **local CI/CD daemon**:  
just `git push` → Fleet pulls, rebuilds, and redeploys on your machine.  

---

<h2>
  <img src="https://github.com/user-attachments/assets/4bf0a9f7-f5b7-4401-9b3d-fc92523cb79c" alt="brain" width="30" height="30"/>
  Summary
</h2>

* [Why Fleet?](#why-fleet)
* [Features](#features)
* [Quick Start](#quick-start)
* [Commands](#commands)
* [Configuration](#configuration)
* [How It Works](#how-it-works)

---

<h2 id="why-fleet">
  <img src="https://i.pinimg.com/originals/84/dc/71/84dc714e0a4c7e1f89d49499ea579db3.gif" alt="brain" width="30" height="30"/>
  Why Fleet?
</h2>


Most CI/CD solutions are:  
- **Cloud-first** → require GitHub, GitLab, or external runners.  
- **Heavyweight** → need databases, web servers, complex setup.  
- **Overkill** for small projects.  

Fleet is different:  
- **Lightweight** → a single Rust binary, no dependencies.  
- **Local-first** → runs directly on your host (perfect for Raspberry Pi, homelab, VPS).  
- **Simple** → configure with a `fleet.yml`, and Fleet takes care of pulling & redeploying.  
- **Connected** → supports notifications (Discord for now, more to come).  


<h2 id="features">
  <img src="https://github.com/user-attachments/assets/dc7fc109-abb2-443a-9bc3-8f6721cdd1e8" alt="brain" width="40" height="40"/>
  Features
</h2>


<img src="https://github.com/user-attachments/assets/7f0beeba-138f-4e43-8c6f-86159bc63cab" width="300" align="right" />

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
# Clone the repository
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
| `fleet watch`           | Add a project to the watch list (run inside the project dir)                           |
| `fleet logs [id\|name]` | Show logs for a project (current dir by default) (`-f` to follow logs)                 |
| `fleet ps`              | List watched projects (`-a` to show stopped projects)                                  |
| `fleet stop <id>`       | Stop watching a project                                                                |
| `fleet up <id>`         | Resume watching a stopped project                                                      |
| `fleet rm <id>`         | Remove a monitored project                                                             |
| `fleet stats`           | Show interactive statistics of all watched projects                                    |
| `fleet run <id>`        | Run a pipeline on demand                                                               |

---


<h2 id="configuration">
  <img src="https://github.com/user-attachments/assets/47ba484c-3bec-43d4-8b50-1e03456709c2" alt="brain" width="30" height="30"/>
  Configuration
</h2>

Each project defines its pipelines with a `fleet.yml` file.

**Example fleet.yml:**

Here’s how I use Fleet to auto-update my Discord bots running on a Raspberry Pi:  

```yaml
ENV: &default_env
  DISCORD_TOKEN: $
  D_WEBHOOK_G: $
  D_WEBHOOK_T: $

branches: ['*'] #you can use wildcard for all or directly a list of branches

timeout: 800

pipeline:
  notifications:
    on: [success, failure]
    thumbnail: https://github.com/user-attachments/assets/429bf6e8-5724-473e-a560-e9e06bbbc143
    channels:
      - service: discord
        url: https://discord.com/api/webhooks/...
  jobs:
    pull:
      steps:
        - cmd: echo "Start update"
        - cmd: git pull
    
    down:
      steps:
        - cmd: docker-compose down
    
    deploy:
      needs: [down, pull]
      env: *default_env
      steps:
        - cmd: docker-compose up --build -d
```

➡️ Now, every time I push to `main`, Fleet automatically pulls the code and redeploys the bot — no manual SSH, no manual Docker restart.  

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
   * Notifications are sent to configured channels (Discord, webhook, etc.).
3. Logs for each project are stored and retrievable via `fleet logs`.
4. Global statistics are available via `fleet stats`.

</details>

---
