# Contributing to FerrFlow

Thanks for your interest in contributing to FerrFlow! Here's how to get started.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/<your-username>/FerrFlow.git`
3. Create a branch: `git checkout -b feat/my-feature`
4. Make your changes
5. Push and open a pull request

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) (nightly toolchain)
- Git

### Build and Test

```bash
cargo build
cargo test
cargo clippy
cargo fmt --check
```

## Guidelines

### Branches

Use conventional prefixes: `feat/`, `fix/`, `refactor/`, `docs/`, `chore/`, `test/`.

One branch per topic. Don't mix unrelated changes.

### Commits

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(config): add hooks support
fix(changelog): handle empty commit list
docs: update CLI reference
```

- Single line, no body
- Scope is optional but recommended
- Breaking changes: add `!` after type/scope (e.g. `feat(config)!: rename field`)

### Pull Requests

- Every PR must reference a GitHub issue. If none exists, create one first.
- PR titles follow the same Conventional Commits format (squash merge uses the title).
- Keep PRs focused. One feature or fix per PR.

### Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix all warnings
- Write tests for new functionality
- Keep functions focused and files reasonable in size

### Documentation

When adding or changing features, update the relevant docs in
`Application/packages/site/src/content/docs/`. Code and documentation ship together.

## Reporting Bugs

Use the [bug report template](https://github.com/FerrLabs/FerrFlow/issues/new?template=bug_report.md).

## Requesting Features

Use the [feature request template](https://github.com/FerrLabs/FerrFlow/issues/new?template=feature_request.md).

## Security

See [SECURITY.md](SECURITY.md) for reporting vulnerabilities.

## License

By contributing, you agree that your contributions will be licensed under the [MPL-2.0 License](LICENSE).
