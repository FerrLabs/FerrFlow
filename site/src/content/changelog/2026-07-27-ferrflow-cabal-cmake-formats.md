---
title: 'FerrFlow · Cabal and CMake version files'
summary: 'FerrFlow now bumps the version in Haskell `*.cabal` files and in the `project()` call of a `CMakeLists.txt`, bringing the supported formats to 16.'
date: 2026-07-27T14:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/773
docsLink: https://ferrflow.com/docs/configuration/formats
---

Two ecosystems that kept coming up are now first-class: Haskell and CMake-based C/C++. Point a versioned file at them and FerrFlow bumps them like any other format, rewriting only the version span so the rest of the file keeps its layout and comments.

```toml
[[package.versioned_files]]
path   = "my-package.cabal"
format = "cabal"

[[package.versioned_files]]
path   = "CMakeLists.txt"
format = "cmake"
```

Both formats are careful about the version-shaped things sitting next to the one you actually want. In a `.cabal` file, `cabal-version:` declares which Cabal feature set the file uses — bumping it would change how the file is parsed — so only the top-level `version:` field is touched, and indented `version:` fields inside stanzas are left alone. In CMake, FerrFlow updates the `VERSION` argument of `project()`, including the multi-line form, while `cmake_minimum_required(VERSION …)` and any `set(<name>_VERSION …)` variable stay untouched.
