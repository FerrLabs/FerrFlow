---
title: Error codes
description: Reference for all FerrFlow error codes with causes and fixes.
---

When FerrFlow encounters an error, it displays a code like `error[E2001]` with a link to this page. Use the code to find the cause and fix.

## Configuration Errors

### E1001: Config file not found

<span id="e1001"></span>

The config file specified via `--config` does not exist.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Run <code>ferrflow init</code> to create a config file, or check the path.</p>
</div></aside>

### E1002: Failed to parse ferrflow.json

<span id="e1002"></span>

The `ferrflow.json` file contains invalid JSON (missing commas, trailing commas, unquoted keys).

### E1003: Failed to parse ferrflow.json5

<span id="e1003"></span>

The `ferrflow.json5` file contains invalid JSON5.

### E1004: Failed to parse ferrflow.toml

<span id="e1004"></span>

The `ferrflow.toml` file contains invalid TOML.

### E1005: Failed to serialize to TOML

<span id="e1005"></span>

Internal error when writing TOML output.

### E1006: Failed to parse .ferrflow

<span id="e1006"></span>

The `.ferrflow` dotfile contains invalid JSON.

### E1007: Failed to serialize .ferrflow

<span id="e1007"></span>

Internal error when writing the dotfile.

### E1008: Failed to resolve path

<span id="e1008"></span>

A path in the config could not be resolved to an absolute path.

### E1009: Failed to write temporary loader file

<span id="e1009"></span>

Could not write the temporary JS/TS loader during config evaluation.

### E1010: Failed to execute tsx

<span id="e1010"></span>

The `tsx` runtime could not be found or executed for `.ts` config files.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Install tsx: <code>npm install -g tsx</code>, or use a JSON/TOML config instead.</p>
</div></aside>

### E1011: Failed to execute node

<span id="e1011"></span>

The `node` runtime could not be found or executed for `.js` config files.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Install Node.js or use a JSON/TOML config instead.</p>
</div></aside>

### E1012: Config evaluation failed

<span id="e1012"></span>

The JS/TS config file threw an error during evaluation.

### E1013: Invalid config output

<span id="e1013"></span>

The JS/TS config file produced non-UTF-8 output.

### E1014: Invalid JSON from config

<span id="e1014"></span>

The JS/TS config file did not produce valid JSON output.

### E1015: Failed to read config file

<span id="e1015"></span>

The config file exists but could not be read (permissions, encoding).

### E1016: Multiple config files found

<span id="e1016"></span>

More than one config file was found in the project root (e.g. both `ferrflow.json` and `ferrflow.toml`).

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Keep only one config file and delete the others.</p>
</div></aside>

### E1017: Config file already exists

<span id="e1017"></span>

Running `ferrflow init` when a config file already exists.

### E1018: Path outside the repository

<span id="e1018"></span>

A `versionedFiles` path is absolute, or climbs out of the repository root with `..`. FerrFlow only reads and writes version files inside the repository, so write the path relative to the repository root.

### E1019: Include pattern matched no file

<span id="e1019"></span>

An entry under `include` matched no file. Patterns resolve relative to the directory of the root config file. Fix the path or remove the pattern.

### E1020: Invalid included file

<span id="e1020"></span>

A file pulled in through `include` is not a valid package description. It either declares `workspace`, `include` or `package`, which only belong in the root config, or its content does not deserialize as a package. An included file describes one package, with the same keys as a `package` entry.

### E1021: Included package outside the repository

<span id="e1021"></span>

The `path` of an included package resolves outside the repository root. In an included file, `path` is relative to that file's directory, and defaults to it when empty.

### E1022: Duplicate package name

<span id="e1022"></span>

Two packages share the same `name`, whether they come from the root config or from `include`. The error names both paths. Rename one of them.

### E1023: Package has no path

<span id="e1023"></span>

A package in the root config has no `path`. Set it relative to the repository root, or move the package into its own file listed under `include`, where `path` defaults to that file's directory.

### E1024: Versioned file does not exist

<span id="e1024"></span>

A package that this run would release lists a `versionedFiles` entry whose file is not on disk. The run stops at plan time rather than at write time, where the same problem surfaces as a bare read error.

The usual cause is a path written relative to the package instead of the repository root. `package.path` is not a prefix that FerrFlow adds for you:

```toml
[[package]]
name = "api"
path = "packages/api"

[[package.versioned_files]]
path = "Cargo.toml"              # wrong, looked up at the repository root
# path = "packages/api/Cargo.toml"  # right
```

The error names the path it probably meant. `ferrflow validate` reports the same problem for every configured package, including ones this run would not touch.

## Validation Errors

### E1100: Invalid repo spec

<span id="e1100"></span>

The `--repo` argument does not match the expected format `owner/repo` or `host/owner/repo`.

### E1101: GitHub API error

<span id="e1101"></span>

The GitHub API returned an error during remote config validation.

### E1102: GitLab API error

<span id="e1102"></span>

The GitLab API returned an error during remote config validation.

### E1103: Invalid UTF-8 in config

<span id="e1103"></span>

The remote config file contains invalid UTF-8 encoding.

### E1104: Failed to parse remote config

<span id="e1104"></span>

The remote config file could not be parsed.

### E1105: Remote config file not found

<span id="e1105"></span>

The specified config file path does not exist in the remote repository.

### E1106: No config file found

<span id="e1106"></span>

No FerrFlow config file was found in the remote repository.

### E1107: --ref requires --repo

<span id="e1107"></span>

The `--ref` flag was used without specifying `--repo`.

## Git Operation Errors

### E2001: Not a git repository

<span id="e2001"></span>

The current directory is not inside a git repository.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Run FerrFlow from within a git repository, or check the <code>--config</code> path.</p>
</div></aside>

### E2002: Bare repository not supported

<span id="e2002"></span>

FerrFlow does not support bare git repositories.

### E2003: Tag already exists

<span id="e2003"></span>

The tag that FerrFlow wants to create already exists.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Delete the existing tag or use <code>--force</code> to overwrite.</p>
</div></aside>

### E2004: Failed to push branch

<span id="e2004"></span>

Could not push the release branch to the remote.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Check that you have push access and the branch is not protected.</p>
</div></aside>

### E2005: Push rejected by remote

<span id="e2005"></span>

The remote rejected the push (non-fast-forward, branch protection, hooks).

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Pull the latest changes and retry, or check branch protection rules.</p>
</div></aside>

### E2006: Failed to push tags

<span id="e2006"></span>

Could not push tags to the remote.

### E2007: Failed to push floating tags

<span id="e2007"></span>

Could not force-push floating tags (e.g. `v1`, `v1.2`).

### E2008: Remote not found

<span id="e2008"></span>

The configured git remote (default: `origin`) does not exist.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Check <code>git remote -v</code> and update the <code>remote</code> field in your config.</p>
</div></aside>

### E2009: Post-push verification failed

<span id="e2009"></span>

After pushing, the release commit could not be verified on the remote branch.

### E2010: Remote branch not found

<span id="e2010"></span>

The remote branch was not found after a push operation.

### E2011: Release already running

<span id="e2011"></span>

Another `ferrflow release` holds the release lock at `.git/ferrflow.lock`, or the lockfile could not be created. A lock left by a dead process on the same machine is taken over automatically, and a lock from another machine once it is more than six hours old. The message prints the lock's content so you can see who owns it.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>If no other release is running, delete the lockfile or rerun with <code>--force-unlock</code>.</p>
</div></aside>

### E2012: Failed to force-push release branch

<span id="e2012"></span>

FerrFlow could not force-push the branch behind the release PR or MR. The message includes git's output. Check that the token can push to that branch and that no protection rule forbids force pushes on it.

### E2013: Cannot inspect release branch

<span id="e2013"></span>

Before reusing the release branch, FerrFlow runs `git ls-remote`, `git fetch` and `git log` on it to check that it only holds commits FerrFlow authored. One of those commands failed. The release does not stop: FerrFlow prints a warning and goes on as if the branch held only its own commits.

### E2014: Failed to delete remote tag

<span id="e2014"></span>

`ferrflow rollback` could not delete, with `git push --delete`, a tag the failed run had pushed. A tag the remote no longer has, or that now points at another commit, is skipped instead. Check that the token can delete tags, then rerun the rollback.

### E2015: Revert failed

<span id="e2015"></span>

`ferrflow rollback` could not `git revert` the release commit, for example because it conflicts with later changes. Finish the revert by hand. FerrFlow never pushes the revert: push the branch yourself once you have checked it.

### E2016: Shadow clone failed

<span id="e2016"></span>

`ferrflow shadow-release` could not clone the repository into a temporary directory. Check that git is on the `PATH` and that the system temp directory is writable.

## GitHub API Errors

### E3001: Failed to create release

<span id="e3001"></span>

The GitHub Releases API returned an error when creating a release.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Check that <code>GITHUB_TOKEN</code> or <code>FERRFLOW_TOKEN</code> has <code>contents: write</code> permission.</p>
</div></aside>

### E3002: Failed to list releases

<span id="e3002"></span>

Could not fetch existing releases from the GitHub API.

### E3003: Failed to parse releases response

<span id="e3003"></span>

The GitHub API returned an unexpected response format.

### E3004: Failed to publish release

<span id="e3004"></span>

Could not publish (un-draft) a GitHub release.

### E3005: Failed to create pull request

<span id="e3005"></span>

The GitHub API returned an error when creating a PR.

### E3006: Failed to parse PR response

<span id="e3006"></span>

The GitHub API returned an unexpected PR response format.

### E3007: PR response missing required field

<span id="e3007"></span>

The GitHub API PR response was missing the `number` or `node_id` field.

### E3008: Failed to enable auto-merge

<span id="e3008"></span>

Could not enable auto-merge on the release PR via the GraphQL API.

### E3009: Failed to parse GraphQL response

<span id="e3009"></span>

The GitHub GraphQL API returned an unexpected response.

### E3010: Auto-merge failed

<span id="e3010"></span>

The GraphQL mutation to enable auto-merge returned an error.

### E3011: Failed to look up release PR

<span id="e3011"></span>

Listing open pull requests failed while FerrFlow looked for an existing release PR to update. Check the token and that the GitHub API is reachable.

### E3012: Failed to update release PR

<span id="e3012"></span>

FerrFlow found the open release PR, but GitHub rejected the update to its title and body. Check that the token has `pull-requests: write`.

### E3013: GraphQL request failed

<span id="e3013"></span>

A request to the GitHub GraphQL API failed before a usable response came back, either on the network or with an HTTP error status. FerrFlow calls this API to author the release commit with `createCommitOnBranch` when `FERRFLOW_BOT` is set.

### E3014: GraphQL error

<span id="e3014"></span>

The GitHub GraphQL API answered with an error, or `createCommitOnBranch` returned no commit id. The message carries GitHub's error text.

### E3015: Failed to move release branch

<span id="e3015"></span>

With `FERRFLOW_BOT` set, FerrFlow points the release branch at the head of the target branch through the GitHub refs API before committing onto it. GitHub rejected the update, or the creation of the ref when the branch did not exist yet. Check that the token has `contents: write`.

### E3016: Failed to delete release

<span id="e3016"></span>

`ferrflow rollback` could not delete a GitHub release the failed run had created. Delete it by hand and rerun the rollback.

## GitLab API Errors

### E3101: Failed to create release

<span id="e3101"></span>

The GitLab Releases API returned an error.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Check that the CI token has API access and the project allows release creation.</p>
</div></aside>

### E3102: Failed to create merge request

<span id="e3102"></span>

The GitLab API returned an error when creating an MR.

### E3103: Failed to parse MR response

<span id="e3103"></span>

The GitLab API returned an unexpected MR response format.

### E3104: MR response missing iid field

<span id="e3104"></span>

The GitLab MR response was missing the `iid` field.

### E3105: Failed to merge MR

<span id="e3105"></span>

Could not merge the release MR via the GitLab API.

### E3106: Failed to look up release MR

<span id="e3106"></span>

Listing open merge requests failed while FerrFlow looked for an existing release MR to update. Check the token and that the GitLab API is reachable.

### E3107: Failed to update release MR

<span id="e3107"></span>

FerrFlow found the open release MR, but GitLab rejected the update to its title and description. Check the token's permissions on the project.

## Gitea API Errors

### E3201: Failed to create release

<span id="e3201"></span>

The Gitea Releases API returned an error when creating a release. Check that the token can write to the repository.

### E3202: Failed to list releases

<span id="e3202"></span>

Could not list releases from the Gitea API while looking for an existing draft release for the tag.

### E3203: Failed to publish release

<span id="e3203"></span>

Could not publish (un-draft) a Gitea release.

## Bitbucket API Errors

### E3301: Failed to resolve release tag

<span id="e3301"></span>

Bitbucket has no releases. On Bitbucket Cloud, FerrFlow looks the new tag up through the API instead, to link to it, and that request failed. Check that the token can read the repository and that the tag was pushed.

## Version File Errors

Each format has its own range of codes. The invalid UTF-8 codes come from `ferrflow validate`, which reads versioned files as raw bytes; every other command reports a file that is not UTF-8 under the format's read error.

### E4101: Cannot read TOML file

<span id="e4101"></span>

FerrFlow could not read the TOML version file (`Cargo.toml`, `pyproject.toml`) from disk.

### E4102: Invalid TOML syntax

<span id="e4102"></span>

The TOML version file does not parse. The message gives the position of the error.

### E4103: No version found in TOML file

<span id="e4103"></span>

None of `package.version`, `workspace.package.version`, `project.version` or `tool.poetry.version` is set. A `Cargo.toml` with `version.workspace = true` also lands here when the same file has no `[workspace.package].version`: point the versioned file at the workspace root `Cargo.toml` instead.

### E4104: Cannot write TOML file

<span id="e4104"></span>

Writing the new version back to the TOML version file (`Cargo.toml`, `pyproject.toml`) failed.

### E4105: Invalid UTF-8 in TOML file

<span id="e4105"></span>

The content of the TOML version file (`Cargo.toml`, `pyproject.toml`) is not valid UTF-8.

### E4201: Cannot read JSON file

<span id="e4201"></span>

FerrFlow could not read the JSON version file (`package.json`, `composer.json`) from disk. The same code covers the release checkpoint `.git/ferrflow.checkpoint.json`.

### E4202: Invalid JSON syntax

<span id="e4202"></span>

The JSON version file does not parse. FerrFlow also raises this code when the release checkpoint `.git/ferrflow.checkpoint.json` is corrupt or was written by another FerrFlow version: delete that file to start fresh.

### E4203: No version found in JSON file

<span id="e4203"></span>

The JSON file has no top-level `version` field.

### E4204: Cannot write JSON file

<span id="e4204"></span>

Writing the new version back to the JSON version file (`package.json`, `composer.json`) failed. FerrFlow also raises it when it cannot save or delete the release checkpoint, or cannot rewrite a dependent package's manifest (`package.json` or `Cargo.toml`) to point at a bumped dependency.

### E4205: Invalid UTF-8 in JSON file

<span id="e4205"></span>

The content of the JSON version file (`package.json`, `composer.json`) is not valid UTF-8.

### E4301: Cannot read Chart.yaml (helm)

<span id="e4301"></span>

FerrFlow could not read the `Chart.yaml` declared with `format = "helm"` from disk.

### E4302: No version found in Chart.yaml (helm)

<span id="e4302"></span>

The `Chart.yaml` declared with `format = "helm"` has no top-level `version:` field.

### E4303: Cannot write Chart.yaml (helm)

<span id="e4303"></span>

Writing the new version back to the `Chart.yaml` declared with `format = "helm"` failed.

### E4304: Invalid UTF-8 in Chart.yaml (helm)

<span id="e4304"></span>

The content of the `Chart.yaml` declared with `format = "helm"` is not valid UTF-8.

### E4401: Cannot read XML file

<span id="e4401"></span>

FerrFlow could not read the XML version file (`pom.xml`) from disk.

### E4402: No version found in XML file

<span id="e4402"></span>

The XML file has no `<version>` tag.

### E4403: Cannot write XML file

<span id="e4403"></span>

Writing the new version back to the XML version file (`pom.xml`) failed.

### E4404: Invalid UTF-8 in XML file

<span id="e4404"></span>

The content of the XML version file (`pom.xml`) is not valid UTF-8.

### E4410: Cannot read .csproj file

<span id="e4410"></span>

FerrFlow could not read the `.csproj` file from disk.

### E4411: No version found in .csproj file

<span id="e4411"></span>

The `.csproj` file has no `<Version>` element.

### E4412: Cannot write .csproj file

<span id="e4412"></span>

Writing the new version back to the `.csproj` file failed.

### E4413: Invalid UTF-8 in .csproj file

<span id="e4413"></span>

The content of the `.csproj` file is not valid UTF-8.

### E4501: Cannot read Gradle file

<span id="e4501"></span>

FerrFlow could not read the Gradle build file (`build.gradle`, `build.gradle.kts`) from disk.

### E4502: No version found in Gradle file

<span id="e4502"></span>

The Gradle build file has no `version = "…"` assignment.

### E4503: Cannot write Gradle file

<span id="e4503"></span>

Writing the new version back to the Gradle build file (`build.gradle`, `build.gradle.kts`) failed.

### E4504: Invalid UTF-8 in Gradle file

<span id="e4504"></span>

The content of the Gradle build file (`build.gradle`, `build.gradle.kts`) is not valid UTF-8.

### E4601: Failed to run git describe

<span id="e4601"></span>

A `gomod` versioned file takes its version from git tags through `git describe`, and the command could not be started. Check that git is on the `PATH`.

### E4602: No version tag found

<span id="e4602"></span>

`git describe` found no tag matching `*@v*` or `v*` for the package.

Starting with FerrFlow v3 the release flow catches this case and falls back to the strategy's bootstrap baseline (`0.0.0` / `0` / …), so the first release on a brand-new repo succeeds without a pre-seeded tag. The error code still exists for programs that import `GoModVersionFile` directly, but end users running `ferrflow release` should not see it anymore.

### E4603: Cannot read a version from go.mod

<span id="e4603"></span>

The version of a Go module comes from git tags, so FerrFlow cannot read one from the content of `go.mod`. `ferrflow validate` skips `gomod` entries with a warning instead.

### E4701: Cannot read text file

<span id="e4701"></span>

FerrFlow could not read the text version file (`VERSION`, `VERSION.txt`) from disk.

### E4702: No version found in text file

<span id="e4702"></span>

The text file is empty. With a `selector`, this code also covers a regex that does not compile, does not have exactly one capture group, matches nothing or captures only whitespace.

### E4703: Cannot write text file

<span id="e4703"></span>

Writing the new version back to the text version file (`VERSION`, `VERSION.txt`) failed.

### E4704: Invalid UTF-8 in text file

<span id="e4704"></span>

The content of the text version file (`VERSION`, `VERSION.txt`) is not valid UTF-8.

### E4801: Cannot read pubspec.yaml

<span id="e4801"></span>

FerrFlow could not read `pubspec.yaml` from disk.

### E4802: No version found in pubspec.yaml

<span id="e4802"></span>

`pubspec.yaml` has no top-level `version:` key.

### E4803: Cannot write pubspec.yaml

<span id="e4803"></span>

Writing the new version back to `pubspec.yaml` failed.

### E4804: Invalid UTF-8 in pubspec.yaml

<span id="e4804"></span>

The content of `pubspec.yaml` is not valid UTF-8.

### E4811: Cannot read mix.exs

<span id="e4811"></span>

FerrFlow could not read `mix.exs` from disk.

### E4812: No version found in mix.exs

<span id="e4812"></span>

`mix.exs` has no `version: "…"` literal.

### E4813: Cannot write mix.exs

<span id="e4813"></span>

Writing the new version back to `mix.exs` failed.

### E4814: Invalid UTF-8 in mix.exs

<span id="e4814"></span>

The content of `mix.exs` is not valid UTF-8.

### E4821: Cannot read Chart.yaml (chartyaml)

<span id="e4821"></span>

FerrFlow could not read the `Chart.yaml` declared with `format = "chartyaml"` from disk.

### E4822: No version found in Chart.yaml (chartyaml)

<span id="e4822"></span>

The `Chart.yaml` declared with `format = "chartyaml"` has no top-level `version:` key.

### E4823: Cannot write Chart.yaml (chartyaml)

<span id="e4823"></span>

Writing the new version back to the `Chart.yaml` declared with `format = "chartyaml"` failed.

### E4824: Invalid UTF-8 in Chart.yaml (chartyaml)

<span id="e4824"></span>

The content of the `Chart.yaml` declared with `format = "chartyaml"` is not valid UTF-8.

### E4831: Cannot read gemspec

<span id="e4831"></span>

FerrFlow could not read the `.gemspec` file from disk.

### E4832: No version found in gemspec

<span id="e4832"></span>

The `.gemspec` file has no `<ident>.version = "…"` assignment.

### E4833: Cannot write gemspec

<span id="e4833"></span>

Writing the new version back to the `.gemspec` file failed.

### E4834: Invalid UTF-8 in gemspec

<span id="e4834"></span>

The content of the `.gemspec` file is not valid UTF-8.

### E4841: Cannot read Package.swift

<span id="e4841"></span>

FerrFlow could not read `Package.swift` from disk.

### E4842: No version found in Package.swift

<span id="e4842"></span>

`Package.swift` has no top-level `let <name>Version = "…"` declaration.

### E4843: Cannot write Package.swift

<span id="e4843"></span>

Writing the new version back to `Package.swift` failed.

### E4844: Invalid UTF-8 in Package.swift

<span id="e4844"></span>

The content of `Package.swift` is not valid UTF-8.

### E4851: Cannot read .cabal file

<span id="e4851"></span>

FerrFlow could not read the `.cabal` file from disk.

### E4852: No version found in .cabal file

<span id="e4852"></span>

The `.cabal` file has no top-level `version:` field.

### E4853: Cannot write .cabal file

<span id="e4853"></span>

Writing the new version back to the `.cabal` file failed.

### E4854: Invalid UTF-8 in .cabal file

<span id="e4854"></span>

The content of the `.cabal` file is not valid UTF-8.

### E4861: Cannot read CMakeLists.txt

<span id="e4861"></span>

FerrFlow could not read `CMakeLists.txt` from disk.

### E4862: No version found in CMakeLists.txt

<span id="e4862"></span>

`CMakeLists.txt` has no `project(… VERSION …)` declaration.

### E4863: Cannot write CMakeLists.txt

<span id="e4863"></span>

Writing the new version back to `CMakeLists.txt` failed.

### E4864: Invalid UTF-8 in CMakeLists.txt

<span id="e4864"></span>

The content of `CMakeLists.txt` is not valid UTF-8.

### E4871: Cannot read galaxy.yml

<span id="e4871"></span>

FerrFlow could not read `galaxy.yml` from disk.

### E4872: No version found in galaxy.yml

<span id="e4872"></span>

`galaxy.yml` has no top-level `version:` key.

### E4873: Cannot write galaxy.yml

<span id="e4873"></span>

Writing the new version back to `galaxy.yml` failed.

### E4874: Invalid UTF-8 in galaxy.yml

<span id="e4874"></span>

The content of `galaxy.yml` is not valid UTF-8.

## Pre-release Errors

### E5001: Empty channel name

<span id="e5001"></span>

The pre-release channel name is empty.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Provide a non-empty channel name: <code>--channel beta</code></p>
</div></aside>

### E5002: Invalid channel name

<span id="e5002"></span>

The channel name contains invalid characters. Only alphanumeric characters and hyphens are allowed.

## Versioning Errors

### E5010: Invalid semver

<span id="e5010"></span>

The current version string is not valid semantic versioning.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Ensure the version in your versioned file follows <code>MAJOR.MINOR.PATCH</code> format.</p>
</div></aside>

## Hook Errors

### E6001: Hook execution failed

<span id="e6001"></span>

A lifecycle hook exited with a non-zero status code and `on_failure` is set to `abort`.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Check the hook command output, or set <code>on_failure: &quot;continue&quot;</code> to ignore failures.</p>
</div></aside>

## Publisher Errors

### E6101: Publisher misconfigured

<span id="e6101"></span>

A publisher could not start because its setup is incomplete: a `registry` that is not declared under `workspace.registries`, a `tokenEnv` variable that is not set, a build context, chart or asset file that does not exist, or a `trustedPublishing` setup outside GitHub Actions. The message names the publisher and the missing piece. Nothing was published.

### E6102: Publish failed

<span id="e6102"></span>

The publish step ran and failed: `cargo publish`, `npm publish`, `twine upload`, `docker buildx`, `helm push`, `gh release upload` or a webhook returned an error. The message carries the tool's own error, including the lines that explain its cause. For cargo, FerrFlow retries a few times when the error looks like registry index lag after a dependency was just published, and reports E6102 once the retries are spent.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Run the same command by hand from the package directory, for example <code>cargo publish --dry-run</code>, to see the full output.</p>
</div></aside>

## Query Errors

### E7001: No packages configured

<span id="e7001"></span>

No packages are defined in the config file.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Run <code>ferrflow init</code> to create a config, or add packages manually.</p>
</div></aside>

### E7002: Package not found

<span id="e7002"></span>

The specified package name does not exist in the config.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Run <code>ferrflow version</code> to list all configured packages.</p>
</div></aside>

### E7003: Invalid diff range

<span id="e7003"></span>

`ferrflow diff` needs a range written `<from>..<to>`, with both sides non-empty, for example `v1.4.0..v1.6.0`.

### E7004: Package name required

<span id="e7004"></span>

The repository is a monorepo, so `ferrflow diff` needs to know which package to compare: `ferrflow diff <package> <from>..<to>`.

### E7005: Range endpoint not found

<span id="e7005"></span>

One side of the `ferrflow diff` range does not resolve to a tag. FerrFlow tries the value as given, then as a version rendered with the package's tag template, with and without a leading `v`. The message lists the names it tried. Pass an existing tag name or version.

## Monorepo Errors

### E8001: Package not found in config

<span id="e8001"></span>

A package referenced during release was not found in the configuration.

### E8002: Floating tag backward move

<span id="e8002"></span>

A floating tag would move to an older version.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Use <code>--force</code> to override the safety check.</p>
</div></aside>

### E8003: Dependency cycle

<span id="e8003"></span>

Two or more packages depend on each other through `dependsOn`, directly or transitively, so there is no order in which to release them. The message names the loop, e.g. `cycle detected: api → web → api`. Remove one of the `dependsOn` edges to break it. The check runs before any version is written, so a cyclic configuration never leaves a partial release behind.

## Rollback Errors

### E10000: Rollback blocked

<span id="e10000"></span>

Every package in the failed run published to a registry that cannot be unpublished, so `ferrflow rollback` has nothing it can undo. The plan it prints names each blocked package and the reason.
