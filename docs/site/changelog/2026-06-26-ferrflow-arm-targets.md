---
title: 'FerrFlow: Windows arm64 and Linux armv7 prebuilt binaries'
summary: 'FerrFlow now ships prebuilt, signed binaries: and matching npm packages: for Windows on ARM (aarch64) and 32-bit ARM Linux (armv7). The release matrix grows from 5 targets to 7, so `npm i ferrflow` and direct downloads now work on Surface/Snapdragon Windows laptops and 32-bit Raspberry Pi / SBCs.'
date: 2026-06-26T10:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/607
docsLink: https://ferrflow.com/docs/verifying-releases/
---

ARM64 was already covered on Linux and macOS, but not Windows: and 32-bit ARM Linux had no build at all. This release closes both gaps, taking the target matrix to seven.

Two new targets ship with every release, signed with Sigstore and listed in the SBOM exactly like the existing five:

- **`aarch64-pc-windows-msvc`** → `ferrflow-windows-arm64.zip` / `@ferrflow/win32-arm64`: Windows on ARM (Surface Pro X/11, Snapdragon X laptops, Dev Kit 2023).
- **`armv7-unknown-linux-musleabihf`** → `ferrflow-linux-armv7.tar.gz` / `@ferrflow/linux-arm`: 32-bit ARM Linux (Raspberry Pi on a 32-bit OS, older SBCs).

The npm wrapper resolves the right binary automatically through `optionalDependencies`, so nothing changes in how you install:

```bash
npm install -D ferrflow
```

On a Windows-on-ARM or armv7 machine npm now pulls the native package instead of failing. Prebuilt archives are also on the [GitHub release page](https://github.com/FerrLabs/FerrFlow/releases), and the GitHub Action installs the matching binary for the runner's architecture.
