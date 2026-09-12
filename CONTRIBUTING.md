# Contributing to Fleet

Thanks for taking the time to contribute! Fleet is a small, focused project and a few simple conventions keep it that way.

## Repository layout

- `src/` is the application crate. The library crate is named `core_lib` (`src/lib.rs`) and the two binaries are `fleet` (CLI client) and `fleetd` (daemon). Each binary re-declares its modules — if you add a new module, declare it in `src/lib.rs` **and** in both binaries.
- `src/config/`, `src/core/`, `src/daemon/`, `src/exec/`, `src/git/`, `src/log/`, `src/notifications/`, `src/cli/` are the functional areas.
- `src/core/docs/*.md` are rustdoc pages embedded with `include_str!` — update them when the corresponding function changes.
- `tests/` contains the integration test suites.
- `fleet.yml` at the repository root is the project's own self-test configuration.

## Required checks

Every pull request must pass, locally and in CI:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --features no-tty --test utils_test --test scheduler_test --test git_test --test daemon_test --test cli_test -- --test-threads=1
cargo build --release
```

### Testing gotchas

- The daemon persists watches to `~/.local/share/fleetd/watches.json` and logs/metrics to `~/.fleet/`. Before running the suite, clear them:

  ```bash
  rm -rf ~/.fleet ~/.local/share/fleetd && mkdir -p ~/.fleet/logs ~/.fleet/metrics
  ```

- Tests must run **from the repository root** and **serially** (`--test-threads=1`): several tests rely on the repo being a git repo with a `fleet.yml` and remote-tracking branches.
- Headless config parsing rejects `$` env placeholders unless the variable exists (`load_config` guards on `stdin` TTY; tests disable that guard with the `no-tty` feature).
- `SECRET_TOKEN` must be set in the environment for `load_config` to parse the root `fleet.yml` in headless mode.

## Git workflow

- Trunk-based: `main` is protected (no direct pushes, PRs required).
- One branch per logical change: `feat/<name>`, `fix/<name>`, `refactor/<name>`, `test/<name>`, `docs/<name>`, `chore/<name>`.
- Conventional commits (`feat(scope): summary`), e.g. `fix(manager): keep state locks off blocking git I/O`.
- Rebase your branch on `main` if it drifts rather than merging `main` into it.
- Keep each commit atomic: no secrets, no build artifacts, no generated files.

## Release process (maintainers)

1. Collect changes in `CHANGELOG.md` under `[Unreleased]`.
2. Bump the version in `Cargo.toml` (and the `Cargo.lock`) with a `chore(release): prepare vX.Y.Z` commit.
3. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`.
4. The `Release` workflow builds the binaries and publishes them to the GitHub Release.