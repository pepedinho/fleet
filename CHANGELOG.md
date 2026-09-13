# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **Daemon no longer freezes on slow or unreachable remotes.** The supervisor used to hold the global state lock across blocking git network calls and branch checkouts; a stalled remote stalled the whole daemon (including the CLI socket) for minutes. Git polling now happens off the async runtime, is bounded by a timeout (`FLEET_GIT_POLL_TIMEOUT_MS`, default 30s) and a concurrency cap (`FLEET_MAX_GIT_POLLS`, default 2). Pipelines run in spawned tasks instead of inline, with per-watch in-flight deduplication.
- Metrics mutex and state lock are no longer held across Discord webhook HTTP calls or `watches.json` / metrics / log disk I/O. Webhook requests are capped at 10s.
- Unix socket connections that connect but never send a request are closed after 5s.
- Commands with an unreportable pid no longer monitor PID 1, which used to turn every run into a spurious timeout.
- Per-job sampling buffer is capped, keeping memory bounded on long-running steps.

### Added
- Release assets now include `fleet-linux-armv7.tar.gz` (Raspberry Pi 2, `armv7-unknown-linux-gnueabihf`), built via `cross` in the release workflow.
- Structured logging via `tracing`, always on: per-watch files are written by a `tracing` layer and events are mirrored to stderr (`RUST_LOG` filtered, default warnings/errors). The `debug-logs` cargo feature is gone.
- Instrumented git poll path (connect / ls-remote / new-commit detection) for daemon diagnosis.
- Regression test: a hanging remote must not block `collect_updates` beyond the poll timeout.
- `cli_test` integration suite is rewritten and enabled in CI (it was entirely commented out).
- `CONTRIBUTING.md`, issue/PR templates, and a release workflow.

### Changed
- `src/daemon/utiles` module renamed to `utils` (`tests/utiles_test.rs` → `tests/utils_test.rs`).
- Bollard pinned to a specific version.
- Debug `println!`/`eprintln!` leftovers removed.
- The missing-env-var warning produced while parsing `fleet.yml` now goes to stderr through `tracing` (previously stdout via `Logger::write`). Per-watch log file format is unchanged.

### Removed
- Live Discord webhook secret from the repository and its entire history (replaced with redacted placeholders; rotate the webhook, the old token is dead).

## [1.0.0] - 2025-??

Initial release of Fleet 1.0: `fleet watch` / `logs` / `ps` / `stop` / `up` / `rm` / `stats` / `run`, `fleetd` daemon, YAML pipelines with dependency graphs, notifications, and statistics.