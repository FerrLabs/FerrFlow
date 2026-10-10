---
title: 'buildMetadata: derive part of the version from your code'
summary: 'A new workspace field naming a command whose output is appended to the version after a plus. The suffix lands on the version files and stops there, so the tag and the changelog keep the plain version.'
date: 2026-09-04T20:00:00Z
product: ferrflow
type: new
draft: true
prLink: https://github.com/FerrLabs/FerrFlow/pull/1022
docsLink: https://ferrflow.com/docs/configuration/config-file/
---

Some versions carry a fact the commits do not know. A Minecraft server advertises the protocol range it speaks, a fork records the upstream revision it was cut from, a firmware build stamps the toolchain that produced it. Semver has a slot for exactly this, the `+suffix` at the end, and until now FerrFlow had no way to fill it: the suffix had to be reattached by a `postBump` hook that reopened the manifest FerrFlow had just written.

`workspace.buildMetadata` names a command instead. FerrFlow runs it once per release from the repository root, before writing anything, and appends its trimmed stdout to the version.

```toml
[workspace]
buildMetadata = "sh scripts/protocol-versions.sh"
```

If the script prints `26.2-26.45`, the manifest reads `1.4.0+26.2-26.45`.

**The suffix goes on the version files and nowhere else.** The tag is `v1.4.0`, the changelog heading is `1.4.0`, and the minor after it is computed from `1.4.0`. That is not a shortcut, it is what semver says: build metadata is excluded from a version's identity and ignored when comparing precedence, so `1.4.0+a` and `1.4.0+b` are the same release. A version that has already been stamped round-trips correctly, so `1.4.0+26.2-26.45` bumps to `1.5.0` and gets stamped afresh.

The output has to be dot-separated alphanumerics and hyphens, which is the full grammar semver allows after the `+`. A command that fails, prints nothing, or prints anything outside that set aborts the release rather than writing a manifest no parser will accept. Under `--dry-run` the command is printed and not executed, in line with how hooks behave.

Reach for it when the derivation is a shell one-liner or a script you already have. If you were doing this with a `postBump` hook, the hook can now shrink to the part that computes the value.
