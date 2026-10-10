---
title: 'FerrFlow: Ansible Galaxy collections get a version file format'
summary: '`galaxy.yml` carries a top-level `version:` like `Chart.yaml` and `pubspec.yaml` do, but versioning one meant falling back to the plain-text format and a hand-written regex. It is a first-class format now.'
date: 2026-09-02T22:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/999
docsLink: https://ferrflow.com/docs/configuration/formats/
---

An Ansible collection declares its version in `galaxy.yml`, the same way a Helm chart declares it in `Chart.yaml`. FerrFlow handled the second and not the first, so a collection had to use the `txt` format and a regex written by hand, which works until someone reformats the file.

`galaxyyml` reads and writes that key directly:

```json
{ "path": "galaxy.yml", "format": "galaxyyml" }
```

`galaxy.yml` is also detected on its own now, so a repository with nothing but a collection manifest gets a working config without naming the format at all.

Everything around the version is left alone. The write replaces the version and nothing else, so `namespace`, `name`, `readme` and any comments come out byte-identical. A `version:` nested under another key, the way a dependency constraint is, is not touched.

Ansible was the one ecosystem gap in a list that already covered Rust, Python, Node, PHP, Java, .NET, Kubernetes, Dart, Elixir, Ruby, Swift, Haskell, C and Go.

Publishing is still yours to run. This versions the manifest; it does not call `ansible-galaxy collection publish`.
