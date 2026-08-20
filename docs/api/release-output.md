# `release --json` output

`ferrflow release --json` emits a single JSON object on stdout describing
the release. All other human-readable stdout is suppressed so the object
is the only thing written (diagnostics and warnings still go to stderr).
This is the machine-readable counterpart to `check --json` and is meant
for CI steps that react to what a release just produced.

Combine it with `--dry-run` to get the computed plan without mutating the
repository.

## Schema

```json
{
  "released": [
    {
      "package": "api",
      "previous_version": "1.2.3",
      "new_version": "1.3.0",
      "bump_type": "minor",
      "tag": "api@v1.3.0",
      "commit_count": 7,
      "prerelease": false,
      "version_source": {
        "kind": "file_over_tag",
        "file": "api/Cargo.toml",
        "tag": "api@v1.2.3"
      },
      "forge_release_url": "https://github.com/owner/repo/releases/tag/api@v1.3.0",
      "forge_release_id": 123456789
    }
  ],
  "skipped": [
    { "package": "web", "reason": "no releasable commits" }
  ],
  "git": {
    "commit": "abc1234",
    "tags_pushed": ["api@v1.3.0"],
    "branch": "main"
  },
  "dry_run": false
}
```

### Fields

| Field | Type | Notes |
| --- | --- | --- |
| `released[]` | array | One entry per package that was (or would be) released this run, including dependency-cascade bumps. |
| `released[].package` | string | Package name from the config. |
| `released[].previous_version` | string | Version before the bump. |
| `released[].new_version` | string | Version after the bump. |
| `released[].bump_type` | string | `major`, `minor`, `patch`, `forced`, or a strategy name (`calver`, `sequential`, …). |
| `released[].tag` | string | The tag that was (or would be) created. |
| `released[].commit_count` | number | Number of commits considered for this package's changelog. |
| `released[].prerelease` | boolean | Whether the new version is a pre-release. |
| `released[].version_source` | object | Where `previous_version` came from. See below. Omitted only when no source could be read at all. |
| `released[].forge_release_url` | string \| null | URL of the created forge release. `null` on dry-run, when no forge is configured, or when the release was not created. |
| `released[].forge_release_id` | number \| null | Numeric id of the created forge release (GitHub). `null` on dry-run, on GitLab, or when not created. |
| `skipped[]` | array | Packages that were considered but not released. |
| `skipped[].package` | string | Package name. |
| `skipped[].reason` | string | Why it was skipped (`not touched`, `no new commits`, `no releasable commits`, `version unchanged`). |
| `git.commit` | string | Short HEAD SHA after the run. |
| `git.tags_pushed` | array | Tags pushed to the remote. Empty on dry-run. |
| `git.branch` | string | Branch the release targeted. |
| `dry_run` | boolean | `true` when `--dry-run` was passed. |

### `version_source`

`previous_version` is resolved from up to two places: the highest tag
reachable from HEAD, and the version written in the package's first
versioned file. `version_source` reports which one it came from, so a
package whose tag was never pushed is distinguishable from one that
simply has no tags yet.

The `_over_` and `_by_policy` kinds both mean two sources were present,
and they answer different questions. `_over_` means the winner was
higher; `_by_policy` means `versionSource` named it and the comparison
never ran. A consumer treating `tag_over_file` and `tag_by_policy` as the
same thing will read a configured choice as evidence the tag was ahead.

| `kind` | Meaning | Extra fields |
| --- | --- | --- |
| `tag` | Only a tag was found. | `tag` |
| `file` | Only the versioned file was found, no tag matched. | `file` |
| `tag_over_file` | Both were found and the tag was the higher of the two. | `tag`, `file` |
| `file_over_tag` | Both were found and the file was the higher of the two. | `file`, `tag` |
| `tag_by_policy` | Both were found and `versionSource: tag` took the tag, whichever was higher. | `tag`, `file` |
| `file_by_policy` | Both were found and `versionSource: file` took the file, whichever was higher. | `file`, `tag` |
| `bootstrap` | Neither was found, the version is the strategy's starting point. | none |

Under the default `versionSource: highest`, when both carry the same
version the tag is credited, matching the resolution order. The same object appears in `check --json` under
`packages[].version_source`, and in `why --json` at the top level.
`why` reports `file` for a package it skipped, because that is the only
source it reads in that case. A member pulled along by a `linked` or `fixed`
group reports `file` for the same reason: its new version comes from the group,
but the previous version it is compared against was read from its own versioned
file. The field is absent only when no source could be read, which for a group
member means the version was borrowed from the group target.

## Dry-run behaviour

With `--dry-run --json`:

- `dry_run` is `true`.
- `released[]` and `skipped[]` are populated from the computed plan.
- `git.tags_pushed` is `[]` and the `forge_release_*` fields are `null`
  (nothing is created or pushed).

## Interaction with `--dry-run --verbose`

`--dry-run --verbose` (without `--json`) prints a unified diff of every
file a release would change, including the changelog. When both `--json`
and `--dry-run --verbose` are passed, `--json` wins: only the JSON object
is emitted and the diffs are suppressed.

## Stability

The field names and structure above are a stable contract. New optional
fields may be added over time; existing fields will not be removed or
renamed without a major version bump. Consumers should ignore unknown
fields rather than failing on them.
