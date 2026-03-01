# Contributing to otel-rs

Thanks for your interest in contributing! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/<you>/otel-rs.git`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Push and open a pull request

## Development Setup

```bash
# Ensure you have Rust 1.93+ installed
rustup update stable

# Check with default features (gRPC)
cargo check

# Check with HTTP transport
cargo check --no-default-features --features "tracing,logs,metrics,http"

# Run tests
cargo test

# Lint
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt -- --check
```

## Code Guidelines

- **Follow existing patterns** — look at similar code in the codebase before writing new code.
- **Error handling** — use `OtelError` variants, never `unwrap()` in library code. Tests may use `unwrap()`.
- **Builder pattern** — all config structs use owned builders with `#[must_use]` on methods.
- **Visibility** — `pub(crate)` for internal modules. Public API re-exported from `lib.rs`.
- **Feature gating** — transport code uses `#[cfg(feature = "grpc")]` / `#[cfg(feature = "http")]`.
- **No type suppression** — never use `as any`, `#[allow(...)]` to hide real issues, or `unsafe` without justification.
- **Formatting** — run `cargo fmt` before committing. Config: 100 char width, 4-space indent.
- **Linting** — `cargo clippy` must pass with `-D warnings`.

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add new sampling strategy
fix: correct timeout handling in HTTP transport
docs: update configuration examples
chore: bump opentelemetry to 0.32
ci: add MSRV check to CI
```

## Pull Requests

- Keep PRs focused — one feature or fix per PR.
- Include tests for new functionality.
- Update documentation if the public API changes.
- Ensure all CI checks pass before requesting review.
- Fill out the PR template.

## Reporting Issues

- Use the **Bug Report** template for bugs.
- Use the **Feature Request** template for enhancements.
- Check existing issues before opening a new one.

## License

By contributing, you agree that your contributions will be dual-licensed under the [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE) licenses.
