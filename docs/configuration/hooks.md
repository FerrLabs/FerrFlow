# Hooks

Hooks are shell commands FerrFlow runs at fixed points in the release
lifecycle. Declare them under `workspace.hooks` (applies to every package) or
under a package's own `hooks` (overrides the workspace value for that package).

```json
{
  "workspace": {
    "hooks": {
      "preCommit": "cargo test --release",
      "postTag": "cargo publish --dry-run",
      "onSuccess": "curl -X POST $DEPLOY_WEBHOOK",
      "onError": "curl -X POST $SLACK_WEBHOOK -d \"release failed: $FERRFLOW_ERROR_CODE\""
    }
  }
}
```

## Lifecycle order

Hooks fire in this order during a release:

| Hook | Fires | Scope |
|------|-------|-------|
| `preBump` | Before version files are written | per package |
| `postBump` | After version files are written, before changelog | per package |
| `preCommit` | After changelog generation, before the git commit | per package |
| `postCommit` | After the release commit is created, before tagging | per package |
| `preTag` | After the commit, immediately before `git tag` | per package |
| `postTag` | After tags are created, before push | per package |
| `prePublish` | After commit and tag, before push | per package |
| `postPublish` | After push and forge release creation | per package |
| `preRelease` | After the release PR is opened, before merge (`releaseCommitMode: pr` only) | once per run |
| `onSuccess` | After the whole release finishes cleanly | once per run |
| `onError` | When the release fails at any git op or aborting hook | once per run |

Per-package hooks run once for each package being released, with that
package's context. Once-per-run hooks (`preRelease`, `onSuccess`, `onError`)
run a single time for the whole invocation.

`postTag` is the natural place to `cargo publish`: the tag exists locally but
has not been pushed yet, so if publishing fails you can drop the local tag and
retry without rewriting remote history.

## Failure handling

`onFailure` is **not** a hook command — it is the strategy for what happens
when a hook exits non-zero:

- `"abort"` (default): stop the release and surface the error.
- `"continue"`: log a warning and carry on.

```json
{ "workspace": { "hooks": { "preBump": "./smoke.sh", "onFailure": "continue" } } }
```

The reactive hook that *runs* when a release fails is `onError` (distinct from
the `onFailure` strategy). It fires once, after the failure, and receives the
failing error code in `FERRFLOW_ERROR_CODE`.

## Context

Every hook runs with these environment variables:

| Variable | Value |
|----------|-------|
| `FERRFLOW_PACKAGE` | Package name (empty for once-per-run hooks) |
| `FERRFLOW_OLD_VERSION` | Version before the bump |
| `FERRFLOW_NEW_VERSION` | Version after the bump |
| `FERRFLOW_BUMP_TYPE` | `major` / `minor` / `patch` / prerelease |
| `FERRFLOW_TAG` | Tag being released (comma-separated list for once-per-run hooks) |
| `FERRFLOW_PACKAGE_PATH` | Absolute path to the package |
| `FERRFLOW_CHANNEL` | Prerelease channel, or empty for a stable release |
| `FERRFLOW_IS_PRERELEASE` | `true` / `false` |
| `FERRFLOW_DRY_RUN` | `true` when running with `--dry-run` |
| `FERRFLOW_ERROR_CODE` | Error code (e.g. `E2005`) — set only for `onError` |

For once-per-run hooks (`preRelease`, `onSuccess`, `onError`) the per-package
variables (`FERRFLOW_PACKAGE`, `FERRFLOW_OLD_VERSION`, …) are empty;
`FERRFLOW_TAG` holds every released tag joined by commas.

`GITHUB_TOKEN`, `FERRFLOW_TOKEN`, and `GITLAB_TOKEN` are stripped from the hook
environment so a hook cannot exfiltrate the release token. A hook that needs a
token must read it from a different variable you inject yourself.

## Dry run

`ferrflow release --dry-run` prints the per-package hooks it *would* run
(prefixed with `⊙`) without executing them. The once-per-run `onSuccess` /
`onError` / `preRelease` hooks depend on a real release outcome and are not
part of the dry-run trace.
