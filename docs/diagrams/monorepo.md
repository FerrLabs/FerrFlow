# Monorepo layout and release

Which package depends on which, and what happens to a tag. Describes `main`; see
[ARCHITECTURE.md](../../ARCHITECTURE.md) for the CI jobs.

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
    API -->|"path, default-features = false"| CLI
    SITE -->|"sync-docs.mjs copies pages,<br/>changelog and schemas"| DOCS
    SITE -.->|"dependsOn, propagate patch"| CLI
```

The API takes the CLI as a path dependency on the root, so both always build from the same commit.

## From a commit to a published artefact

```mermaid
flowchart TB
    PUSH(["push to main"]) --> TESTS["ci.yml: test, fixtures, benchmarks"]
    TESTS --> REL["Release job: ferrflow release --draft"]
    REL --> CFG[".ferrflow: 3 packages,<br/>nestedPackages = exclusive"]
    CFG --> TCLI["tag v7.x.y"]
    CFG --> TSITE["tag site@vYYYY.M.N"]
    TCLI -->|"cascade: site dependsOn ferrflow"| TSITE
    CFG --> TAPI["tag api@vYYYY.M.N"]
    TCLI --> PUB["publish.yml"]
    PUB --> OUT["release assets, crates.io, npm,<br/>@ferrflow/wasm, @ferrflow/doc,<br/>ghcr.io/ferrlabs/ferrflow"]
    TSITE --> DOCK["docker.yml"]
    TAPI --> DOCK
    DOCK --> IMG["ghcr.io/ferrlabs/ferrflow-cloud/site or api<br/>tags X.Y.Z and X.Y"]
    IMG --> REL2["publish the draft release<br/>(--latest=false)"]
```

A push that only touches `docs/site/**` does not start `ci.yml`, so it cannot reach the Release job.
