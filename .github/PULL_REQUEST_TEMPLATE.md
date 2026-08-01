## Summary

<!-- One or two sentences: what does this PR change? -->

## Motivation

<!-- Why is this change needed? Link issues with `Fixes #123` / `Closes #456`. -->

## What changed

<!-- Bullet list of concrete changes. -->

-

## Tests

<!-- How did you verify this works? -->

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --release` passes (in-crate unit tests)
- [ ] If new feature: at least one unit test added inline in the module
- [ ] If rendering change: PNG output attached to PR description for before/after

## Screenshots / output

<!-- For rendering / layout changes, paste the PNG that Falco produces. -->

## Checklist

- [ ] Updated `README.md` if user-facing behavior changed
- [ ] Updated `docs/` if architecture changed
- [ ] No `unwrap()` / `panic!()` added in hot paths
- [ ] Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
