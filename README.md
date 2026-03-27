# FerrFlow

[![CI](https://github.com/FerrFlow-Org/FerrFlow/actions/workflows/ci.yml/badge.svg)](https://github.com/FerrFlow-Org/FerrFlow/actions/workflows/ci.yml)
[![Release](https://github.com/FerrFlow-Org/FerrFlow/actions/workflows/release.yml/badge.svg)](https://github.com/FerrFlow-Org/FerrFlow/actions/workflows/release.yml)
[![Latest release](https://img.shields.io/github/v/release/FerrFlow-Org/FerrFlow)](https://github.com/FerrFlow-Org/FerrFlow/releases/latest)
[![License](https://img.shields.io/github/license/FerrFlow-Org/FerrFlow)](LICENSE)

Universal semantic versioning for monorepos and classic repos.

FerrFlow reads your commit history, determines the right version bump, updates your version files, generates a changelog, and creates a tagged release — for any language, any repo layout.

## Why FerrFlow?

Most versioning tools are tied to a specific ecosystem (semantic-release for JS, cargo-release for Rust) or require a Node.js runtime. FerrFlow is a single compiled binary with no runtime dependencies.

| Tool | Monorepo | Multi-language | Runtime |
|------|----------|---------------|---------|
| semantic-release | plugins | JS only | Node.js |
| changesets | manual | JS only | Node.js |
| knope | limited | partial | none |
| FerrFlow | native | yes | none |

## Supported version files

| Format | File | Ecosystem |
|--------|------|-----------|
| TOML | `Cargo.toml` | Rust |
| TOML | `pyproject.toml` | Python |
| JSON | `package.json` | Node.js |
| XML | `pom.xml` | Java / Maven |

## Installation

**Cargo**

```bash
cargo install ferrflow
```

**npm**

```bash
npm install -D ferrflow
```

**Docker**

```bash
docker run ghcr.io/ferrflow/ferrflow:latest check
```

**Pre-built binaries**

Download from [Releases](https://github.com/FerrFlow/FerrFlow/releases).

## Usage

```bash
# Preview what would be bumped
ferrflow check

# Run a release
ferrflow release

# Dry run
ferrflow release --dry-run

# Scaffold a config file
ferrflow init
```

## Configuration

FerrFlow reads `ferrflow.toml` at the root of your repository. If no config file is found, it auto-detects common version files in the current directory.

### Single package

```toml
[workspace]
remote = "origin"
branch = "main"

[[package]]
name = "my-app"
path = "."
changelog = "CHANGELOG.md"

[[package.versioned_files]]
path = "Cargo.toml"
format = "toml"
```

### Monorepo

```toml
[[package]]
name = "api"
path = "services/api"
changelog = "services/api/CHANGELOG.md"
shared_paths = ["services/shared/"]  # bump this package when shared/ changes

[[package.versioned_files]]
path = "services/api/Cargo.toml"
format = "toml"

[[package]]
name = "frontend"
path = "frontend"
changelog = "frontend/CHANGELOG.md"

[[package.versioned_files]]
path = "frontend/package.json"
format = "json"
```

## Conventional Commits

FerrFlow follows the [Conventional Commits](https://www.conventionalcommits.org/) spec.

| Prefix | Bump |
|--------|------|
| `fix:`, `perf:`, `refactor:` | patch |
| `feat:` | minor |
| `feat!:`, `BREAKING CHANGE` | major |
| `chore:`, `docs:`, `ci:` | none |

## CI usage

**GitLab CI**

```yaml
release:
  image: ghcr.io/ferrflow/ferrflow:latest
  script:
    - ferrflow release
  rules:
    - if: '$CI_COMMIT_BRANCH == "main"'
```

**GitHub Actions**

```yaml
- name: Release
  run: ferrflow release
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## License

MIT
