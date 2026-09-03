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

## Version File Errors

### TOML (E4101 to E4105)

| Code      | Error                             |
| --------- | --------------------------------- |
| **E4101** | Cannot read TOML version file     |
| **E4102** | Invalid TOML syntax               |
| **E4103** | No `version` field found          |
| **E4104** | Failed to write TOML version file |
| **E4105** | Invalid UTF-8 in TOML file        |

### JSON (E4201 to E4205)

| Code      | Error                             |
| --------- | --------------------------------- |
| **E4201** | Cannot read JSON version file     |
| **E4202** | Invalid JSON syntax               |
| **E4203** | No `version` field found          |
| **E4204** | Failed to write JSON version file |
| **E4205** | Invalid UTF-8 in JSON file        |

### Helm / YAML (E4301 to E4304)

| Code      | Error                       |
| --------- | --------------------------- |
| **E4301** | Cannot read Chart.yaml      |
| **E4302** | No `version` field found    |
| **E4303** | Failed to write Chart.yaml  |
| **E4304** | Invalid UTF-8 in Chart.yaml |

### XML (E4401 to E4404)

| Code      | Error                        |
| --------- | ---------------------------- |
| **E4401** | Cannot read XML version file |
| **E4402** | No `<version>` tag found     |
| **E4403** | Failed to write XML file     |
| **E4404** | Invalid UTF-8 in XML file    |

### CSProj (E4410 to E4413)

| Code      | Error                         |
| --------- | ----------------------------- |
| **E4410** | Cannot read .csproj file      |
| **E4411** | No `<Version>` tag found      |
| **E4412** | Failed to write .csproj file  |
| **E4413** | Invalid UTF-8 in .csproj file |

### Gradle (E4501 to E4504)

| Code      | Error                         |
| --------- | ----------------------------- |
| **E4501** | Cannot read build.gradle      |
| **E4502** | No `version` field found      |
| **E4503** | Failed to write build.gradle  |
| **E4504** | Invalid UTF-8 in build.gradle |

### Go mod (E4601 to E4603)

| Code      | Error                           |
| --------- | ------------------------------- |
| **E4601** | Failed to run `git describe`    |
| **E4602** | No version tag found            |
| **E4603** | Go modules do not support write |

### Text (E4701 to E4704)

| Code      | Error                         |
| --------- | ----------------------------- |
| **E4701** | Cannot read text version file |
| **E4702** | No version found              |
| **E4703** | Failed to write text file     |
| **E4704** | Invalid UTF-8 in text file    |

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

## Monorepo Errors

### E8001: Package not found in config

<span id="e8001"></span>

A package referenced during release was not found in the configuration.

### E8002: Floating tag backward move

<span id="e8002"></span>

A floating tag would move to an older version.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Use <code>--force</code> to override the safety check.</p>
</div></aside>
