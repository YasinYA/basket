# Contributing

Thanks for your interest in contributing to Basket.

## Quick Start

```bash
cargo fmt
cargo check
cargo run
```

## Code Style

- Use `cargo fmt` before submitting.
- Keep changes focused and easy to review.
- Prefer small, well‑scoped PRs.

## Pre-commit tests

We run tests before each commit locally. Install the hook:

```bash
ln -s ../../scripts/pre-commit-test.sh .git/hooks/pre-commit
```

If you already have a pre-commit hook, merge the command or run `cargo test` before committing.

## Reporting Issues

Please include:
- What you expected vs what happened
- Steps to reproduce
- Your OS and Rust version

## Pull Requests

- Describe the problem and the fix.
- Mention any user‑facing changes.
- Add or update tests if applicable.

## Security

If you find a security issue, see `SECURITY.md`.
