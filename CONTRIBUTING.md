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

### Fuzzing

The parsers read files and commit messages that come from someone else's
repository, so they are fuzzed. The targets live in `fuzz/`, which is its own
crate outside the workspace.

```bash
cargo install cargo-fuzz
cargo +nightly fuzz list
cargo +nightly fuzz run config_parse fuzz/corpus/config_parse fuzz/seeds/config_parse -- -max_total_time=60
```

`cargo-fuzz` needs a nightly toolchain, and `rust-toolchain.toml` pins stable,
so the `+nightly` is not optional. On Windows there is no libFuzzer runtime to
link against; use WSL, a container, or let CI do it.

An input that breaks a target is written to `fuzz/artifacts/<target>/`. Replay
it with `cargo +nightly fuzz run <target> <that file>`, and commit it under
`fuzz/seeds/<target>/` with the fix so it stays covered.

CI fuzzes every target for 45 seconds on a pull request that touches `src/` or
`fuzz/`, and for ten minutes a night on `main`. The corpus is cached between
runs, so the nightly run starts where the last one left off.

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

Developer notes live in `docs/`. The pages rendered at
[ferrflow.com/docs](https://ferrflow.com/docs) live in `docs/site/`, published as
`@ferrflow/doc` on each release tag and consumed by the site as a dependency.

When you add or change a feature, update `docs/site/docs-en/` in the same pull request. That is the
point of keeping them here: a flag and the sentence describing it are reviewed together and cannot
drift apart. The French pages under `docs/site/docs-fr/` are allowed to lag, and a PR is not blocked
for leaving them.

Use `docs:` for a change that only touches documentation. Anything else would bump the CLI and cut a
release, publishing a binary to crates.io and eight npm packages for a typo, and none of that can be
withdrawn. A pull request that ships code and its documentation together takes the code's type, as
usual.

Never edit `docs/site/docs-vN/` or `docs/site/docs-fr-vN/`. Those record what a past major actually
documented, mistakes included, because someone pinned to that major reads them to understand the
binary they are running. CI rejects a pull request that modifies one, and allows adding a new one.

## Reporting Bugs

Use the [bug report template](https://github.com/FerrLabs/FerrFlow/issues/new?template=bug_report.md).

## Requesting Features

Use the [feature request template](https://github.com/FerrLabs/FerrFlow/issues/new?template=feature_request.md).

## Security

See [SECURITY.md](SECURITY.md) for reporting vulnerabilities.

### Supply-chain audits

CI runs `cargo audit`, `cargo deny`, `cargo machete`, and `cargo vet` on every
PR. `cargo vet` checks that every dependency in `Cargo.lock` is either audited
by a trusted source (Mozilla and the Bytecode Alliance — imported in
`supply-chain/config.toml`) or explicitly exempted.

When you add or bump a dependency, `cargo vet` will fail until the new crate is
accounted for. To resolve it locally:

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

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
