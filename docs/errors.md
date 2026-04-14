# FerrFlow Error Reference

This file is the source-of-truth registry for all FerrFlow error codes.
Each code is stable: once assigned, it is never reused for a different error.

## Configuration Errors (E1000--E1099)

### E1001: Config file not found

The config file specified via `--config` does not exist.

**Fix**: Check the file path, or run `ferrflow init` to create a config file.

### E1002: Failed to parse ferrflow.json

The `ferrflow.json` file contains invalid JSON.

**Fix**: Validate the JSON syntax (missing commas, trailing commas, unquoted keys).

### E1003: Failed to parse ferrflow.json5

The `ferrflow.json5` file contains invalid JSON5.

### E1004: Failed to parse ferrflow.toml

The `ferrflow.toml` file contains invalid TOML.

### E1005: Failed to serialize to TOML

Internal error when writing TOML output.

### E1006: Failed to parse .ferrflow

The `.ferrflow` dotfile contains invalid JSON.

### E1007: Failed to serialize .ferrflow

Internal error when writing the dotfile.

### E1008: Failed to resolve path

A path in the config could not be resolved to an absolute path.

### E1009: Failed to write temporary loader file

Could not write the temporary JS/TS loader during config evaluation.

### E1010: Failed to execute tsx

The `tsx` runtime could not be found or executed for `.ts` config files.

**Fix**: Install tsx (`npm install -g tsx`) or use a JSON/TOML config instead.

### E1011: Failed to execute node

The `node` runtime could not be found or executed for `.js` config files.

**Fix**: Install Node.js or use a JSON/TOML config instead.

### E1012: Config evaluation failed

The JS/TS config file threw an error during evaluation.

**Fix**: Check the config file for syntax errors and ensure it exports a valid config object.

### E1013: Invalid config output

The JS/TS config file produced non-UTF-8 output.

### E1014: Invalid JSON from config

The JS/TS config file did not produce valid JSON output.

### E1015: Failed to read config file

The config file exists but could not be read (permissions, encoding).

### E1016: Multiple config files found

More than one config file was found in the project root (e.g. both `ferrflow.json` and `ferrflow.toml`).

**Fix**: Keep only one config file.

### E1017: Config file already exists

Running `ferrflow init` when a config file already exists.

## Validation Errors (E1100--E1199)

### E1100: Invalid repo spec

The `--repo` argument does not match the expected format `owner/repo` or `host/owner/repo`.

### E1101: GitHub API error

The GitHub API returned an error during remote config validation.

### E1102: GitLab API error

The GitLab API returned an error during remote config validation.

### E1103: Invalid UTF-8 in config

The remote config file contains invalid UTF-8 encoding.

### E1104: Failed to parse remote config

The remote config file could not be parsed.

### E1105: Remote config file not found

The specified config file path does not exist in the remote repository.

### E1106: No config file found

No FerrFlow config file was found in the remote repository.

### E1107: --ref requires --repo

The `--ref` flag was used without specifying `--repo`.

## Git Operation Errors (E2000--E2099)

### E2001: Not a git repository

The current directory (or specified path) is not inside a git repository.

**Fix**: Run FerrFlow from within a git repository, or check the path.

### E2002: Bare repository not supported

FerrFlow does not support bare git repositories.

### E2003: Tag already exists

The tag that FerrFlow wants to create already exists.

**Fix**: Delete the existing tag or use `--force` to overwrite.

### E2004: Failed to push branch

Could not push the release branch to the remote.

**Fix**: Check that you have push access and the branch is not protected.

### E2005: Push rejected by remote

The remote rejected the push (non-fast-forward, branch protection, hooks).

**Fix**: Pull the latest changes and retry, or check branch protection rules.

### E2006: Failed to push tags

Could not push tags to the remote.

### E2007: Failed to push floating tags

Could not force-push floating tags (e.g. `v1`, `v1.2`).

### E2008: Remote not found

The configured git remote (default: `origin`) does not exist.

**Fix**: Check `git remote -v` and update the `remote` field in your config.

### E2009: Post-push verification failed

After pushing, the release commit could not be verified on the remote branch.

### E2010: Remote branch not found after push

The remote branch was not found after a push operation.

## GitHub API Errors (E3000--E3099)

### E3001: Failed to create GitHub release

The GitHub Releases API returned an error when creating a release.

**Fix**: Check that `GITHUB_TOKEN` or `FERRFLOW_TOKEN` has `contents: write` permission.

### E3002: Failed to list GitHub releases

Could not fetch existing releases from the GitHub API.

### E3003: Failed to parse releases response

The GitHub API returned an unexpected response format.

### E3004: Failed to publish GitHub release

Could not publish (un-draft) a GitHub release.

### E3005: Failed to create pull request

The GitHub API returned an error when creating a PR.

### E3006: Failed to parse PR response

The GitHub API returned an unexpected PR response format.

### E3007: PR response missing required field

The GitHub API PR response was missing the `number` or `node_id` field.

### E3008: Failed to enable auto-merge

Could not enable auto-merge on the release PR via the GraphQL API.

### E3009: Failed to parse GraphQL response

The GitHub GraphQL API returned an unexpected response.

### E3010: Auto-merge failed

The GraphQL mutation to enable auto-merge returned an error.

## GitLab API Errors (E3100--E3199)

### E3101: Failed to create GitLab release

The GitLab Releases API returned an error.

**Fix**: Check that the CI token has API access and the project allows release creation.

### E3102: Failed to create merge request

The GitLab API returned an error when creating an MR.

### E3103: Failed to parse MR response

The GitLab API returned an unexpected MR response format.

### E3104: MR response missing iid field

The GitLab MR response was missing the `iid` field.

### E3105: Failed to merge MR

Could not merge the release MR via the GitLab API.

## Version File Errors (E4000--E4799)

### TOML (E4101--E4105)

- **E4101**: Cannot read TOML version file
- **E4102**: Invalid TOML syntax
- **E4103**: No `version` field found in TOML file
- **E4104**: Failed to write TOML version file
- **E4105**: Invalid UTF-8 in TOML file

### JSON (E4201--E4205)

- **E4201**: Cannot read JSON version file
- **E4202**: Invalid JSON syntax
- **E4203**: No `version` field found in JSON file
- **E4204**: Failed to write JSON version file
- **E4205**: Invalid UTF-8 in JSON file

### Helm/YAML (E4301--E4304)

- **E4301**: Cannot read Chart.yaml
- **E4302**: No `version` field found in Chart.yaml
- **E4303**: Failed to write Chart.yaml
- **E4304**: Invalid UTF-8 in Chart.yaml

### XML (E4401--E4404)

- **E4401**: Cannot read XML version file
- **E4402**: No `<version>` tag found
- **E4403**: Failed to write XML version file
- **E4404**: Invalid UTF-8 in XML file

### CSProj (E4410--E4413)

- **E4410**: Cannot read .csproj file
- **E4411**: No `<Version>` tag found
- **E4412**: Failed to write .csproj file
- **E4413**: Invalid UTF-8 in .csproj file

### Gradle (E4501--E4504)

- **E4501**: Cannot read build.gradle
- **E4502**: No `version` field found
- **E4503**: Failed to write build.gradle
- **E4504**: Invalid UTF-8 in build.gradle

### Go mod (E4601--E4603)

- **E4601**: Failed to run `git describe` for Go module version
- **E4602**: No version tag found for Go module
- **E4603**: Go modules do not support write operations

### Text (E4701--E4704)

- **E4701**: Cannot read text version file
- **E4702**: No version found in text file
- **E4703**: Failed to write text version file
- **E4704**: Invalid UTF-8 in text file

## Pre-release Errors (E5000--E5099)

### E5001: Empty channel name

The pre-release channel name is empty.

**Fix**: Provide a non-empty channel name (e.g. `--channel beta`).

### E5002: Invalid channel name

The channel name contains invalid characters (only alphanumeric and hyphens allowed).

## Versioning Errors (E5010--E5019)

### E5010: Invalid semver

The current version string is not valid semantic versioning.

**Fix**: Ensure the version in your versioned file follows `MAJOR.MINOR.PATCH` format.

## Hook Errors (E6000--E6099)

### E6001: Hook execution failed

A lifecycle hook exited with a non-zero status code and `on_failure` is set to `abort`.

**Fix**: Check the hook command output and fix the underlying issue, or set `on_failure: "continue"` to ignore failures.

## Query Errors (E7000--E7099)

### E7001: No packages configured

No packages are defined in the config file.

**Fix**: Run `ferrflow init` to create a config, or add packages manually.

### E7002: Package not found

The specified package name does not exist in the config.

**Fix**: Check the package name with `ferrflow version` to list all configured packages.

## Monorepo Errors (E8000--E8099)

### E8001: Package not found in config

A package referenced during release was not found in the configuration.

### E8002: Floating tag backward move

A floating tag would move to an older version. Use `--force` to override.
