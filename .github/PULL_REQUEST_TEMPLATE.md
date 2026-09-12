## Summary

<!-- What does this PR do, in one or two sentences? Link the issue if any. -->

## Type of change

- [ ] feat
- [ ] fix
- [ ] refactor
- [ ] test
- [ ] docs
- [ ] chore

## Checks

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --features no-tty --test utils_test --test scheduler_test --test git_test --test daemon_test --test cli_test -- --test-threads=1` (run from the repo root, state cleared)
- [ ] `cargo build --release`
- [ ] Updated `CONVENTION` documentation if behavior changed (`src/core/docs/*.md`)

## Notes

<!-- Anything reviewers should know: trade-offs, migration impact, follow-up work. -->