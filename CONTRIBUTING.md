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

## Git hooks

Run once after cloning:

```bash
./.githooks/install.sh
```

That sets `git config core.hooksPath .githooks` so:

- **pre-commit** runs `cargo fmt --check` + `cargo clippy -D warnings` on every commit that touches Rust files.
- **pre-push** runs `cargo test --workspace --all-features` so broken code never reaches the remote.

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

### Supply-chain audits

CI runs `cargo audit`, `cargo deny`, `cargo machete`, and `cargo vet` on every
PR. `cargo vet` checks that every dependency in `Cargo.lock` is either audited
by a trusted source (Mozilla, Google, the Bytecode Alliance — imported in
`supply-chain/config.toml`) or explicitly exempted.

When you add or bump a dependency, `cargo vet --locked` will fail until the new
crate is accounted for. To resolve it locally:

```bash
cargo install cargo-vet   # once
cargo vet                  # shows what's missing
cargo vet certify          # record your own audit of a crate you reviewed
# or, to trust it on faith for now:
cargo vet add-exemption <crate> <version>
```

Commit the resulting changes to `supply-chain/`. Prefer a real audit
(`certify`) for small crates you can read; use an exemption when a full review
isn't practical. Run `cargo vet prune` periodically to drop exemptions that
imported audits now cover.

Release artifacts carry [SLSA build provenance](https://ferrflow.com/docs/verifying-releases/#slsa-build-provenance);
verify a downloaded binary with `gh attestation verify`.

## License

By contributing, you agree that your contributions will be licensed under the [MPL-2.0 License](LICENSE).
