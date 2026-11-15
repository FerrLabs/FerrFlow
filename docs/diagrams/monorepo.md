# Monorepo layout and release

Which package depends on which, and what happens to a tag. Describes `main`; see
[ARCHITECTURE.md](../../ARCHITECTURE.md) for the pending CI changes.

## Packages and dependencies

```mermaid
flowchart LR
    subgraph MIT["MIT"]
        CLI["ferrflow<br/>crate + binary + npm<br/>root"]
        WASM["ferrflow-wasm"]
        DOCS["docs/site<br/>@ferrflow/doc"]
    end
    subgraph MPL["MPL-2.0"]
        API["api<br/>ferrflow-api"]
        SITE["site<br/>@ferrflow/site"]
    end
    CRATES[("crates.io<br/>ferrflow 7.x")]

    WASM -->|path| CLI
    CLI -->|published as| CRATES
    API -->|"version = 7, default-features = false"| CRATES
    SITE -->|"sync-docs.mjs copies pages,<br/>changelog and schemas"| DOCS
```

The API takes the CLI from crates.io today. FLOW-10 (PR 1409) turns that edge into a path dependency on the root.

## From a commit to a published artefact

```mermaid
flowchart TB
    PUSH(["push to main"]) --> TESTS["ci.yml: test, fixtures, benchmarks"]
    TESTS --> REL["Release job: ferrflow release --draft"]
    REL --> CFG[".ferrflow: 3 packages,<br/>nestedPackages = exclusive"]
    CFG --> TCLI["tag v7.x.y"]
    CFG --> TSITE["tag site@vYYYY.M.N"]
    CFG --> TAPI["tag api@vYYYY.M.N"]
    TCLI --> PUB["publish.yml"]
    PUB --> OUT["release assets, crates.io, npm,<br/>@ferrflow/wasm, @ferrflow/doc,<br/>ghcr.io/ferrlabs/ferrflow"]
    TSITE --> NONE1["no workflow on main"]
    TAPI --> NONE2["no workflow on main"]
```

A push that only touches `docs/site/**` does not start `ci.yml`, so it cannot reach the Release job.
