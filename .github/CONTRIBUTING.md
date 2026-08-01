# Contributing to Falco

Thanks for your interest in improving Falco! This is a small, focused
project, so the rules below are correspondingly small.

## Getting started

1. Fork the repo and create a feature branch.
2. `cargo build --release` — should produce `target/release/falco`.
3. `cargo test --release` — should pass all in-crate unit tests.
4. Pick an issue (or file one first if you're not sure).

## What we accept

- **Bug fixes** — always welcome, especially rendering / parsing
  correctness.
- **CSS / HTML / JS feature support** — please open an issue first so
  we can decide on scope, then PR against that issue.
- **Performance improvements** — please include a before/after
  benchmark.
- **Tests** — always welcome, even for things that already work.

## What we probably won't accept

- Large dependency additions. Falco is intentionally small (~10 MB
  binary). If your feature requires a 5 MB crate, please discuss first.
- Rewrites of working modules "for cleanliness" without a concrete
  user-visible benefit.
- Spec compliance changes that break existing rendering without a
  fallback.

## Before opening a PR

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test --release` passes (in-crate unit tests)
- [ ] If a new feature — at least one unit test added inline in the module
- [ ] If a rendering change — PNG output attached to the PR for review

## Commit message style

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(layout): support CSS Grid auto-flow dense
fix(css): inherit line-height as unitless when source is unitless
docs(readme): add terminal usage example
chore(deps): bump ureq to 2.10
```

## Architecture overview

See the [README architecture table](../README.md#architecture) for the
module breakdown. The high-level pipeline is:

```
HTML -> DOM -> Style tree -> Layout tree -> Paint commands -> Canvas -> PNG / Window
```

Major modules:

| Module            | Purpose                                                |
| ----------------- | ------------------------------------------------------ |
| `html5/`          | WHATWG tokenizer + tree builder (the spec-compliant one) |
| `html.rs`         | Legacy HTML parser (still used in the render pipeline)  |
| `dom/`, `dom2/`   | Legacy DOM + spec-compliant DOM (live links, observers) |
| `css/`, `css2/`   | CSS parser + selectors level 4 + cascade                |
| `style/`          | Cascade + inheritance + UA stylesheet                  |
| `layout/`         | Block / inline / flex / table / grid / float / absolute |
| `paint/`          | Canvas + font rasterizer + compositing                 |
| `tjs/`, `tjs_ext/`| Custom JS VM + Promise/Symbol/BigInt/Generator         |
| `web_runtime/`    | fetch / XHR / event loop / video / webgl / promise      |
| `security/`       | SOP / CSP / sandbox / cert / permissions / DevTools    |
| `window/`         | Interactive window mode (events, scrolling, forms)     |
| `net/`            | HTTP fetch, cookies, cache, websocket, redirect        |

When adding a feature, look for the matching module. If you're unsure,
open an issue and ask.

## Adding tests

- **Unit tests** live in the same file as the code, in a `#[cfg(test)]`
  module — Falco does not ship a separate `tests/` directory or
  `benchmarks/` directory.
- **Rendering tests** should attach the PNG to the PR (we don't yet
  have automated visual diffing).

## License

By contributing, you agree that your contributions will be licensed
under the MIT license.
