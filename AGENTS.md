# AGENTS.md

Rust CI/CD orchestrator: a `fleet` CLI talking to a `fleetd` daemon over a Unix socket. Local-first replacement for cloud CI.

## Build & verify (order matters)

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings` (CI fails on any warning)
- `cargo test --features no-tty --test utiles_test --test scheduler_test --test git_test --test daemon_test -- --test-threads=1`
- `cargo build --release`

Rust edition 2024, stable toolchain; no `rust-toolchain` file (CI uses `dtolnay/rust-toolchain@stable`).

## Git workflow

- Trunk-based: `main` is the only long-lived branch and must stay green. Every change goes through a short-lived `feature/*`/`chore/*`/`fix/*`/`refactor/*`/`docs/*` branch + PR into `main`; no direct pushes to `main` (branch protection on).
- Conventional commit messages: `type(scope): description` (`feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`, `build`). One logical change per commit.
- Each PR must pass the full verify suite (below) and be reviewed before merge. Releases are tagged versions on `main` (`vX.Y.Z`) — no `develop` branch.
- Branch protection rule on `main` (GitHub Settings → Branches): require PR review, require status checks (`fmt`, `clippy`, `test`, `build`), block direct pushes, require up-to-date branches.

## Testing gotchas

- Tests MUST run serial (`--test-threads=1`) — they share on-disk state.
- Tests write state to `~/.local/share/fleetd/watches.json`, logs to `~/.fleet/logs`, metrics to `~/.fleet/metrics`. Before running locally, clear these (`rm -rf ~/.local/share/fleetd ~/.fleet`) or tests pick up stale state; CI clears them first.
- `--features no-tty` is required for headless test runs; `stdin_is_tty()` (config/mod.rs:117) forces false under this feature.
- `tests/cli_test.rs` is entirely commented out — do not enable it blindly; `fleet.yml` (repo root) runs it via `cargo test --test cli_test -- --test-threads=1`.

## Architecture (not obvious from filenames)

- Crate layout: lib crate `core_lib` (`src/lib.rs`) + two binaries `fleet` (`src/fleet.rs`, CLI) and `fleetd` (`src/fleetd.rs`, daemon).
- **Trap:** each binary re-declares every module with `mod cli; mod config; ...`. Any new module/`pub mod` must be declared in `src/lib.rs` AND both binaries.
- `fleetd` = supervisor loop + Unix socket server (`src/daemon/server.rs`); `fleet` builds `DaemonRequest` JSON and sends it over the socket (`src/cli/client.rs`).
- Persistent registry is `AppState::load_from_disk`/`save_to_disk` → `watches.json` (src/core/state.rs); each watch is a `WatchContext` (src/core/watcher.rs) holding repo + `fleet.yml` config.
- Pipelines: `run_pipeline` (src/exec/pipeline.rs) resolves `needs` deps, detects cycles; steps can run in Docker (`container:`), use `pipe`/`blocking`, per-step `env`.
- `force_commit` feature makes `watch_once` (src/core/watcher.rs:107) always report a new commit — handy for local testing.
- rustdoc source-of-truth: `src/core/docs/*.md` embedded via `#[doc = include_str!]` and deployed to GitHub Pages via `.github/workflows/docs.yml`.

## Project lifecycle

- `fleet watch` inside a repo dir registers a watch; `fleet ps/logs/stop/up/rm/run/stats` manage it.
- `make install` = release build + systemd **user** service for `fleetd`; `make update` rebuilds + restarts; `make uninstall` removes all (service + `~/.config/fleet`).
- Repo-root `fleet.yml` is the project's own self-test config — keep, it's referenced by tests; don't add secrets beyond what's there.
- PR descriptions should follow the before/after format in `.github/copilot-instructions.md`.