import type { HookFunction } from './hook-context.js';

export type Publisher =
  | {
      kind: 'cargo';
      registry?: string;
      allowDirty?: boolean;
      /**
       * Pass cargo publish --no-verify. Set for multi-crate batch releases so inter-dependent crates don't fail on registry-index propagation timing.
       */
      noVerify?: boolean;
      /**
       * Extra flags appended verbatim to the underlying cargo publish invocation. Escape hatch for options FerrFlow doesn't model natively.
       */
      args?: string[];
      /**
       * Snake_case form of `allowDirty` — both spellings are accepted.
       */
      allow_dirty?: boolean;
      /**
       * Snake_case form of `noVerify` — both spellings are accepted. Pass cargo publish --no-verify. Set for multi-crate batch releases so inter-dependent crates don't fail on registry-index propagation timing.
       */
      no_verify?: boolean;
    }
  | {
      kind: 'npm';
      registry?: string;
      tag?: string;
      access?: 'public' | 'restricted';
      /**
       * Extra flags appended verbatim to the underlying npm publish invocation. Escape hatch for options FerrFlow doesn't model natively.
       */
      args?: string[];
    }
  | {
      kind: 'docker';
      image: string;
      tags?: string[];
      platforms?: string[];
      context?: string;
      dockerfile?: string;
      sign?: 'none' | 'sigstore';
      /**
       * Extra flags appended verbatim to the underlying docker buildx build invocation (before the build-context positional). Escape hatch for options FerrFlow doesn't model natively.
       */
      args?: string[];
    }
  | {
      kind: 'helm';
      chart?: string;
      registry: string;
      /**
       * Extra flags appended verbatim to the underlying helm push invocation. Escape hatch for options FerrFlow doesn't model natively.
       */
      args?: string[];
    }
  | {
      kind: 'github-release-asset';
      path: string;
      displayName?: string;
      /**
       * Extra flags appended verbatim to the underlying gh release upload invocation. Escape hatch for options FerrFlow doesn't model natively.
       */
      args?: string[];
    }
  | {
      kind: 'pypi';
      /**
       * Name of an entry under workspace.registries. Its url becomes twine --repository-url and its tokenEnv supplies TWINE_PASSWORD with username __token__. Omit to publish to pypi.org.
       */
      registry?: string;
      /**
       * Run python -m build in the package directory before uploading. Set false when the distributions in dist/ are produced earlier in the pipeline.
       */
      build?: boolean;
      /**
       * Mint a short-lived upload token from PyPI trusted publishing instead of reading a stored token. Requires a GitHub Actions job with permissions: id-token: write, and this repository registered as a trusted publisher for the project. Mutually exclusive with the registry's tokenEnv.
       */
      trustedPublishing?: boolean;
      /**
       * Snake_case form of `trustedPublishing` — both spellings are accepted. Mint a short-lived upload token from PyPI trusted publishing instead of reading a stored token. Requires a GitHub Actions job with permissions: id-token: write, and this repository registered as a trusted publisher for the project. Mutually exclusive with the registry's tokenEnv.
       */
      trusted_publishing?: boolean;
      /**
       * Extra flags appended verbatim to the underlying twine upload invocation.
       */
      args?: string[];
    }
  | {
      kind: 'webhook';
      url: string;
      body?: unknown;
      headers?: {
        [k: string]: string;
      };
    };

/**
 * Configuration file for FerrFlow — universal semantic versioning for monorepos and classic repos.
 */
export interface FerrFlowConfiguration {
  /**
   * JSON Schema reference for editor autocompletion.
   */
  $schema?: string;
  /**
   * Files that each declare one package, merged into `package`. Paths and globs are relative to this config file. A package path inside an included file is relative to that file, and defaults to its directory.
   */
  include?: string[];
  /**
   * Workspace-level settings.
   */
  workspace?: {
    /**
     * Git remote name.
     */
    remote?: string;
    /**
     * Main branch name.
     */
    branch?: string;
    /**
     * @deprecated
     * Deprecated and ignored: telemetry was removed from FerrFlow in v5.33. The key remains accepted so existing configs written before then stay valid, but it drives nothing and should be deleted.
     */
    anonymous_telemetry?: boolean;
    /**
     * @deprecated
     * Deprecated and ignored: telemetry was removed from FerrFlow in v5.33. The key remains accepted so existing configs written before then stay valid, but it drives nothing and should be deleted.
     */
    telemetry?: boolean;
    /**
     * Default versioning strategy for all packages.
     */
    versioning?: 'semver' | 'calver' | 'calver-short' | 'calver-seq' | 'calver-short-seq' | 'sequential' | 'zerover';
    /**
     * Version format built from named variables. Variables: year, short_year, month, padded_month, day, padded_day, week, padded_week, quarter, seq, major, minor, patch. Each must be separated by a literal so the previous version can be read back. The literals cannot carry whitespace, control characters, or the characters git reserves in a ref name (~ ^ : ? * [ backslash), because the version becomes a git tag. Takes precedence over 'versioning'.
     */
    versionTemplate?: string;
    /**
     * Tag template. Use {name} for package name and {version} for version. Default: 'v{version}' for single repos, '{name}@v{version}' for monorepos.
     */
    tagTemplate?: string;
    /**
     * Compare files against the last tag instead of just the last commit, recovering missed releases in monorepos.
     */
    recoverMissedReleases?: boolean;
    /**
     * How to commit version bumps and changelog updates. 'commit' pushes directly to the branch, 'pr' creates a pull request, 'none' skips committing entirely.
     */
    releaseCommitMode?: 'commit' | 'pr' | 'none';
    /**
     * In monorepo mode, whether to create a single grouped commit for all packages or one commit per package. Only affects behavior when multiple packages are bumped.
     */
    releaseCommitScope?: 'grouped' | 'per-package';
    /**
     * What to put in the body of the release commit message, below the subject line. 'none' keeps the single-line subject. 'summary' lists one line per released package with its commit count. 'full' embeds the changelog section written for each package.
     */
    releaseCommitBody?: 'none' | 'summary' | 'full';
    /**
     * Which commit subjects map to which bump level. Resolution is major > minor > patch, first match wins. Breaking markers (feat!:, fix(api)!:, a BREAKING CHANGE: footer) are always detected structurally, whatever is configured here.
     */
    commitFormats?: {
      /**
       * Extra patterns that force a major bump. Empty by default — structural breaking markers already cover the conventional forms.
       */
      major?: string | string[];
      /**
       * Patterns that select a minor bump. Defaults cover feat:, feat(scope):, feature:, and the Title-case and branch-style (Feat/) variants.
       */
      minor?: string | string[];
      /**
       * Patterns that select a patch bump. Defaults cover fix:, perf:, refactor: and their Title-case and branch-style variants.
       */
      patch?: string | string[];
      /**
       * Match patterns exactly as written. Set false to lowercase both pattern and subject before matching. The 'all' catch-all keyword is recognised in any casing regardless.
       */
      caseSensitive?: boolean;
    };
    /**
     * Automatically merge release PRs when releaseCommitMode is 'pr'.
     */
    autoMergeReleases?: boolean;
    /**
     * Append [skip ci] to the release commit message. Defaults to true in 'commit' mode, false in 'pr' mode.
     */
    skipCi?: boolean;
    /**
     * Markers in a commit subject line that cause FerrFlow to skip the commit when computing the next version. Matched case-insensitively, subject line only. Defaults to ['[skip ci]', '[ci skip]', '[no ci]', '[skip actions]', '[actions skip]'].
     */
    commitSkipMarkers?: string[];
    /**
     * Floating tag levels to create/move on each release. Each level produces a tag pointing to the latest matching release (e.g. 'major' creates a v1 tag for v1.2.3).
     */
    floatingTags?: ('major' | 'minor')[];
    /**
     * Git forge type. 'auto' detects from the remote URL. Set 'github', 'gitlab', 'gitea', or 'bitbucket' to override (useful for self-hosted instances with custom domains).
     */
    forge?: 'auto' | 'github' | 'gitlab' | 'gitea' | 'bitbucket';
    /**
     * Template for a floating alias tag that always points at this package's newest non-prerelease release, e.g. "latest" or "{name}@latest". Absent (default) means no alias tag. Deliberately separate from tagTemplate: the alias is a name, not a version, so a tagTemplate of "v{version}" does not turn it into "vlatest". In a monorepo the template must include {name}, or every package will fight over the same ref.
     */
    latestTag?: string;
    /**
     * How to handle tags pointing to orphaned commits (after rebase + force-push). 'warn' logs a warning and skips. 'treeHash' attempts recovery by matching the commit's tree hash. 'message' attempts recovery by matching the commit message.
     */
    orphanedTagStrategy?: 'warn' | 'treeHash' | 'message';
    /**
     * Which source wins when a package has both a git tag and a version in a versioned file. 'highest' (default) takes whichever is higher, so a mistake in either source ratchets the version upward and is never walked back. 'tag' treats the tags as the record of what shipped and ignores the file. 'file' treats the file as the source and ignores the tag, which is what a package migrated between repos usually needs. With only one source present the setting has no effect.
     */
    versionSource?: 'highest' | 'tag' | 'file';
    hooks?: Hooks;
    /**
     * When true, `ferrflow release` does not run the configured publishers inline — it defers them to a separate `ferrflow publish` invocation (e.g. a CI job that carries the docker/helm/npm build toolchain the release job lacks). `ferrflow publish` always runs the publishers regardless of this flag.
     */
    deferPublish?: boolean;
    /**
     * Opt-in single-file source of truth for per-package versions, relative to the repo root (e.g. '.ferrflow.manifest.json'). When set, `ferrflow release` writes a full version snapshot to this file as part of the release commit, validates it against the versioned files at release start (a divergence is a hard error — run `ferrflow sync-manifest` to repair), and `status` / `version` read current versions from it. Omit to leave all behaviour unchanged.
     */
    manifest_file?: string;
    /**
     * In a monorepo where one package's path contains another's, which packages a file under the inner path counts for. 'shared' (default) counts it for both, so a change in a nested package also bumps the package that contains it. 'exclusive' gives it to the package with the longest matching path only, so a nested package owns its subtree. `sharedPaths` still count as an explicit opt-in in both modes.
     */
    nestedPackages?: 'shared' | 'exclusive';
    /**
     * When true, after `ferrflow release` bumps a package, every dependent listed via `dependsOn` has its version constraint for that package rewritten to the new version in its own manifest, and the manifest is staged in the release commit. Only `json` (package.json) and `toml` (Cargo.toml) manifests are rewritten, and only plain operator + version constraints (`^1.2.3`, `~1.2.3`, `1.2.3`) — a `workspace:*`, `file:`/`git:` spec or a multi-part range is left untouched for a human to handle.
     */
    updateDependents?: boolean;
    /**
     * When true, after `ferrflow release` bumps a manifest (Cargo.toml, package.json, pyproject.toml, Gemfile, mix.exs) it refreshes the sibling lockfile (Cargo.lock, package-lock.json / pnpm-lock.yaml / yarn.lock, poetry.lock / uv.lock, Gemfile.lock, mix.lock) with the package manager's offline / lockfile-only mode and stages it in the same release commit. A missing package manager or an unresolvable offline update is warned about, never fatal. Set per-package `updateLockfiles: false` to opt a single package out.
     */
    updateLockfiles?: boolean;
    /**
     * Named registry credentials referenced by package.publishers[]. Lets users declare 'the Kellnr token lives in CARGO_REGISTRIES_KELLNR_TOKEN' once and reuse it across every cargo / npm / docker publisher.
     */
    registries?: {
      [k: string]: {
        url?: string;
        /**
         * Name of the env var holding the registry token. The token value is never read from config.
         */
        tokenEnv?: string;
        /**
         * Snake_case form of `tokenEnv` — both spellings are accepted. Name of the env var holding the registry token. The token value is never read from config.
         */
        token_env?: string;
      };
    };
    /**
     * Map branches to pre-release channels. When FerrFlow runs on a branch matching a configured name/pattern, it uses the corresponding channel.
     */
    branches?: {
      /**
       * Branch name or glob pattern (e.g. 'main', 'release/*').
       */
      name: string;
      /**
       * false for stable releases, string for pre-release channel name.
       */
      channel?: false | string;
      /**
       * Strategy for the pre-release identifier after the channel name.
       */
      prereleaseIdentifier?: 'increment' | 'timestamp' | 'short-hash' | 'timestamp-hash';
      /**
       * Snake_case form of `prereleaseIdentifier` — both spellings are accepted. Strategy for the pre-release identifier after the channel name.
       */
      prerelease_identifier?: 'increment' | 'timestamp' | 'short-hash' | 'timestamp-hash';
    }[];
    /**
     * Opt-in changelog rendering. When this whole block is omitted, FerrFlow renders the classic sections (Breaking Changes, Features, Bug Fixes, Refactoring) with flat bullets and no links; perf commits fold into Bug Fixes, and every other type, docs and security included, is left out.
     */
    changelog?: {
      /**
       * Map commit-type prefixes to section labels, or false to hide. Any conventional commit type works, including chore, ci, build and test; feat, fix, perf, security, docs and refactor carry built-in default labels and render first, in that order, with the rest following alphabetically. When omitted, defaults to feat -> Features and fix -> Bug Fixes (Breaking Changes always shown). 'security' applies to security: and fix(security): commits. A breaking marker always wins over the type's section.
       */
      sections?: {
        [k: string]: string | false;
      };
      /**
       * Group bullets by commit scope using an inline bold prefix (- **api:** add events endpoint). Scopeless commits render first.
       */
      groupByScope?: boolean;
      /**
       * Append a commit link to each bullet (([abc1234](<forge>/commit/abc1234))) when the forge is known. Silently omitted when the remote forge cannot be resolved.
       */
      includeCommitLinks?: boolean;
      /**
       * Emit a keep-a-changelog compare footer ([1.2.3]: <forge>/compare/v1.2.2...v1.2.3) when the forge is known and a previous tag exists.
       */
      includeCompareLink?: boolean;
      /**
       * Snake_case form of `groupByScope` — both spellings are accepted. Group bullets by commit scope using an inline bold prefix (- **api:** add events endpoint). Scopeless commits render first.
       */
      group_by_scope?: boolean;
      /**
       * Snake_case form of `includeCommitLinks` — both spellings are accepted. Append a commit link to each bullet (([abc1234](<forge>/commit/abc1234))) when the forge is known. Silently omitted when the remote forge cannot be resolved.
       */
      include_commit_links?: boolean;
      /**
       * Snake_case form of `includeCompareLink` — both spellings are accepted. Emit a keep-a-changelog compare footer ([1.2.3]: <forge>/compare/v1.2.2...v1.2.3) when the forge is known and a previous tag exists.
       */
      include_compare_link?: boolean;
    };
    /**
     * Snake_case form of `tagTemplate` — both spellings are accepted. Tag template. Use {name} for package name and {version} for version. Default: 'v{version}' for single repos, '{name}@v{version}' for monorepos.
     */
    tag_template?: string;
    /**
     * Snake_case form of `recoverMissedReleases` — both spellings are accepted. Compare files against the last tag instead of just the last commit, recovering missed releases in monorepos.
     */
    recover_missed_releases?: boolean;
    /**
     * Snake_case form of `releaseCommitMode` — both spellings are accepted. How to commit version bumps and changelog updates. 'commit' pushes directly to the branch, 'pr' creates a pull request, 'none' skips committing entirely.
     */
    release_commit_mode?: 'commit' | 'pr' | 'none';
    /**
     * Snake_case form of `releaseCommitScope` — both spellings are accepted. In monorepo mode, whether to create a single grouped commit for all packages or one commit per package. Only affects behavior when multiple packages are bumped.
     */
    release_commit_scope?: 'grouped' | 'per-package';
    /**
     * Snake_case form of `autoMergeReleases` — both spellings are accepted. Automatically merge release PRs when releaseCommitMode is 'pr'.
     */
    auto_merge_releases?: boolean;
    /**
     * Snake_case form of `skipCi` — both spellings are accepted. Append [skip ci] to the release commit message. Defaults to true in 'commit' mode, false in 'pr' mode.
     */
    skip_ci?: boolean;
    /**
     * Snake_case form of `commitSkipMarkers` — both spellings are accepted. Markers in a commit subject line that cause FerrFlow to skip the commit when computing the next version. Matched case-insensitively, subject line only. Defaults to ['[skip ci]', '[ci skip]', '[no ci]', '[skip actions]', '[actions skip]'].
     */
    commit_skip_markers?: string[];
    /**
     * Snake_case form of `floatingTags` — both spellings are accepted. Floating tag levels to create/move on each release. Each level produces a tag pointing to the latest matching release (e.g. 'major' creates a v1 tag for v1.2.3).
     */
    floating_tags?: ('major' | 'minor')[];
    /**
     * Snake_case form of `orphanedTagStrategy` — both spellings are accepted. How to handle tags pointing to orphaned commits (after rebase + force-push). 'warn' logs a warning and skips. 'treeHash' attempts recovery by matching the commit's tree hash. 'message' attempts recovery by matching the commit message.
     */
    orphaned_tag_strategy?: 'warn' | 'treeHash' | 'message';
    /**
     * Snake_case form of `versionSource`, both spellings are accepted. Which source wins when a package has both a git tag and a version in a versioned file. 'highest' (default) takes whichever is higher, so a mistake in either source ratchets the version upward and is never walked back. 'tag' treats the tags as the record of what shipped and ignores the file. 'file' treats the file as the source and ignores the tag, which is what a package migrated between repos usually needs. With only one source present the setting has no effect.
     */
    version_source?: 'highest' | 'tag' | 'file';
    /**
     * Snake_case form of `deferPublish` — both spellings are accepted. When true, `ferrflow release` does not run the configured publishers inline — it defers them to a separate `ferrflow publish` invocation (e.g. a CI job that carries the docker/helm/npm build toolchain the release job lacks). `ferrflow publish` always runs the publishers regardless of this flag.
     */
    defer_publish?: boolean;
    /**
     * Snake_case form of `nestedPackages`, both spellings are accepted. In a monorepo where one package's path contains another's, which packages a file under the inner path counts for. 'shared' (default) counts it for both, so a change in a nested package also bumps the package that contains it. 'exclusive' gives it to the package with the longest matching path only, so a nested package owns its subtree. `sharedPaths` still count as an explicit opt-in in both modes.
     */
    nested_packages?: 'shared' | 'exclusive';
    /**
     * Snake_case form of `updateDependents` — both spellings are accepted. When true, after `ferrflow release` bumps a package, every dependent listed via `dependsOn` has its version constraint for that package rewritten to the new version in its own manifest, and the manifest is staged in the release commit. Only `json` (package.json) and `toml` (Cargo.toml) manifests are rewritten, and only plain operator + version constraints (`^1.2.3`, `~1.2.3`, `1.2.3`) — a `workspace:*`, `file:`/`git:` spec or a multi-part range is left untouched for a human to handle.
     */
    update_dependents?: boolean;
    /**
     * Snake_case form of `updateLockfiles` — both spellings are accepted. When true, after `ferrflow release` bumps a manifest (Cargo.toml, package.json, pyproject.toml, Gemfile, mix.exs) it refreshes the sibling lockfile (Cargo.lock, package-lock.json / pnpm-lock.yaml / yarn.lock, poetry.lock / uv.lock, Gemfile.lock, mix.lock) with the package manager's offline / lockfile-only mode and stages it in the same release commit. A missing package manager or an unresolvable offline update is warned about, never fatal. Set per-package `updateLockfiles: false` to opt a single package out.
     */
    update_lockfiles?: boolean;
    /**
     * Groups of packages that share a version line when co-released. When any member of a group has a releasable commit, every member is bumped to the same version (the highest resulting version across the group). Package names stay distinct. Each inner array is one group and must list at least two package names; a package may appear in only one linked/fixed group.
     */
    linked?: string[][];
    /**
     * Groups of packages locked to an identical version forever. Behaves like `linked`, but is intended for packages that must never diverge; `ferrflow validate` warns when a fixed group's versions have drifted apart. Each inner array is one group and must list at least two package names; a package may appear in only one linked/fixed group.
     */
    fixed?: string[][];
    /**
     * Command whose stdout is appended to the version as semver build metadata, after a `+`. It is run once per release, never in a dry run, and its output must be dot-separated alphanumerics and hyphens. The tag and the changelog keep the plain version, since build metadata is excluded from version identity and ignored for precedence.
     */
    buildMetadata?: string;
    /**
     * Snake_case form of `buildMetadata` — both spellings are accepted. Command whose stdout is appended to the version as semver build metadata, after a `+`. It is run once per release, never in a dry run, and its output must be dot-separated alphanumerics and hyphens. The tag and the changelog keep the plain version, since build metadata is excluded from version identity and ignored for precedence.
     */
    build_metadata?: string;
    /**
     * Command whose stdout is the version this release ships, replacing the one the versioning strategy computed. It receives the usual `FERRFLOW_*` variables, with `FERRFLOW_NEW_VERSION` holding the computed proposal. It runs only in `release` (dry run included), after the bump is decided and before any version file, tag or changelog heading is written. `check`, `preview`, `status`, `why` and `graph` never run it and report the version as set by the command. The output must be a version that goes past the current one and does not collide with an existing tag. Not allowed on a package in a linked or fixed group.
     */
    versionCommand?: string;
    /**
     * Snake_case form of `versionCommand` — both spellings are accepted. Command whose stdout is the version this release ships, replacing the one the versioning strategy computed. It receives the usual `FERRFLOW_*` variables, with `FERRFLOW_NEW_VERSION` holding the computed proposal. It runs only in `release` (dry run included), after the bump is decided and before any version file, tag or changelog heading is written. `check`, `preview`, `status`, `why` and `graph` never run it and report the version as set by the command. The output must be a version that goes past the current one and does not collide with an existing tag. Not allowed on a package in a linked or fixed group.
     */
    version_command?: string;
  };
  /**
   * List of packages to version. Use one entry for single repos, multiple for monorepos.
   */
  package?: {
    /**
     * Package name, used in tags and changelog headers.
     */
    name: string;
    /**
     * Path to the package root, relative to the repo root. Use "." for single repos.
     */
    path: string;
    /**
     * Path to the changelog file, relative to the repo root.
     */
    changelog?: string | null;
    /**
     * Files that contain the package version to update on release.
     */
    versionedFiles?: {
      /**
       * Path to the version file, relative to the repo root.
       */
      path: string;
      /**
       * File format used to locate and update the version field.
       */
      format:
        | 'csproj'
        | 'toml'
        | 'json'
        | 'xml'
        | 'gradle'
        | 'gomod'
        | 'helm'
        | 'chartyaml'
        | 'pubspecyaml'
        | 'galaxyyml'
        | 'mixexs'
        | 'gemspec'
        | 'packageswift'
        | 'cabal'
        | 'cmake'
        | 'txt';
      /**
       * Optional selector to disambiguate which version is bumped. xml: a slash-delimited tag path like '/project/version' (or '//tag' for first-anywhere); txt: a regex with one capture group around the version. Leading and trailing whitespace inside the capture is trimmed from the version and left in the file, so a padded line keeps its padding across releases; a capture that is only whitespace is an error. Without a selector, xml uses a Maven-aware default (first <version> direct child of root).
       */
      selector?: string;
    }[];
    /**
     * Paths whose changes should trigger a version bump for this package (monorepo only).
     */
    sharedPaths?: string[];
    /**
     * Packages this package depends on. When one is bumped, this package is bumped too — by default with the same bump type. Each entry is either a package name or an object with a `propagate` policy.
     */
    dependsOn?: (
      | string
      | {
          name: string;
          /**
           * How this dependency's bump becomes a bump here. same: identical bump (a breaking upstream makes this package breaking too). major-on-major: major stays major, anything lower becomes a patch. patch: always a patch. none: no cascade from this dependency.
           */
          propagate?: 'same' | 'major-on-major' | 'patch' | 'none';
        }
    )[];
    /**
     * Versioning strategy for this package. Overrides workspace default.
     */
    versioning?: 'semver' | 'calver' | 'calver-short' | 'calver-seq' | 'calver-short-seq' | 'sequential' | 'zerover';
    /**
     * Version format built from named variables. Variables: year, short_year, month, padded_month, day, padded_day, week, padded_week, quarter, seq, major, minor, patch. Each must be separated by a literal so the previous version can be read back. The literals cannot carry whitespace, control characters, or the characters git reserves in a ref name (~ ^ : ? * [ backslash), because the version becomes a git tag. Overrides workspace default. Takes precedence over 'versioning'.
     */
    versionTemplate?: string;
    /**
     * Tag template for this package. Use {name} for package name and {version} for version. Overrides workspace default.
     */
    tagTemplate?: string;
    /**
     * Floating tag levels for this package. Overrides workspace default.
     */
    floatingTags?: ('major' | 'minor')[];
    /**
     * Floating alias tag template for this package. Overrides workspace default. Absent means the workspace setting applies.
     */
    latestTag?: string;
    /**
     * Overrides `workspace.buildMetadata` for this package. A command replaces the workspace one, `true` inherits it, and `false` releases this package with a plain version while the rest of the workspace stays stamped.
     */
    buildMetadata?: string | boolean;
    /**
     * Snake_case form of `buildMetadata` — both spellings are accepted. Overrides `workspace.buildMetadata` for this package. A command replaces the workspace one, `true` inherits it, and `false` releases this package with a plain version while the rest of the workspace stays stamped.
     */
    build_metadata?: string | boolean;
    /**
     * Overrides `workspace.versionCommand` for this package. Command whose stdout is the version this release ships, replacing the one the versioning strategy computed. It receives the usual `FERRFLOW_*` variables, with `FERRFLOW_NEW_VERSION` holding the computed proposal. It runs only in `release` (dry run included), after the bump is decided and before any version file, tag or changelog heading is written. `check`, `preview`, `status`, `why` and `graph` never run it and report the version as set by the command. The output must be a version that goes past the current one and does not collide with an existing tag. Not allowed on a package in a linked or fixed group.
     */
    versionCommand?: string;
    /**
     * Snake_case form of `versionCommand` — both spellings are accepted. Overrides `workspace.versionCommand` for this package. Command whose stdout is the version this release ships, replacing the one the versioning strategy computed. It receives the usual `FERRFLOW_*` variables, with `FERRFLOW_NEW_VERSION` holding the computed proposal. It runs only in `release` (dry run included), after the bump is decided and before any version file, tag or changelog heading is written. `check`, `preview`, `status`, `why` and `graph` never run it and report the version as set by the command. The output must be a version that goes past the current one and does not collide with an existing tag. Not allowed on a package in a linked or fixed group.
     */
    version_command?: string;
    hooks?: Hooks;
    /**
     * Declarative publish targets evaluated after the GitHub Release is created. Each entry is a self-contained intent — see $defs/publisher for the kinds.
     */
    publishers?: Publisher[];
    /**
     * Per-package override for workspace.updateLockfiles. Set false to skip lockfile refresh for this package even when the workspace default is on; set true to refresh only this package when the workspace default is off. Omit to inherit the workspace setting.
     */
    updateLockfiles?: boolean;
    /**
     * Per-package override for workspace.versionSource. Lets one package in a monorepo pick its own tag-vs-file resolution without changing the policy for the others. Omit to inherit the workspace setting.
     */
    versionSource?: 'highest' | 'tag' | 'file';
    /**
     * Snake_case form of `versionedFiles` — both spellings are accepted. Files that contain the package version to update on release.
     */
    versioned_files?: {
      /**
       * Path to the version file, relative to the repo root.
       */
      path: string;
      /**
       * File format used to locate and update the version field.
       */
      format:
        | 'csproj'
        | 'toml'
        | 'json'
        | 'xml'
        | 'gradle'
        | 'gomod'
        | 'helm'
        | 'chartyaml'
        | 'pubspecyaml'
        | 'galaxyyml'
        | 'mixexs'
        | 'gemspec'
        | 'packageswift'
        | 'cabal'
        | 'cmake'
        | 'txt';
      /**
       * Optional selector to disambiguate which version is bumped. xml: a slash-delimited tag path like '/project/version' (or '//tag' for first-anywhere); txt: a regex with one capture group around the version. Leading and trailing whitespace inside the capture is trimmed from the version and left in the file, so a padded line keeps its padding across releases; a capture that is only whitespace is an error. Without a selector, xml uses a Maven-aware default (first <version> direct child of root).
       */
      selector?: string;
    }[];
    /**
     * Snake_case form of `sharedPaths` — both spellings are accepted. Paths whose changes should trigger a version bump for this package (monorepo only).
     */
    shared_paths?: string[];
    /**
     * Snake_case form of `dependsOn` — both spellings are accepted. Packages this package depends on. When one is bumped, this package is bumped too — by default with the same bump type. Each entry is either a package name or an object with a `propagate` policy.
     */
    depends_on?: (
      | string
      | {
          name: string;
          /**
           * How this dependency's bump becomes a bump here. same: identical bump (a breaking upstream makes this package breaking too). major-on-major: major stays major, anything lower becomes a patch. patch: always a patch. none: no cascade from this dependency.
           */
          propagate?: 'same' | 'major-on-major' | 'patch' | 'none';
        }
    )[];
    /**
     * Snake_case form of `tagTemplate` — both spellings are accepted. Tag template for this package. Use {name} for package name and {version} for version. Overrides workspace default.
     */
    tag_template?: string;
    /**
     * Snake_case form of `floatingTags` — both spellings are accepted. Floating tag levels for this package. Overrides workspace default.
     */
    floating_tags?: ('major' | 'minor')[];
    /**
     * Snake_case form of `updateLockfiles` — both spellings are accepted. Per-package override for workspace.updateLockfiles. Set false to skip lockfile refresh for this package even when the workspace default is on; set true to refresh only this package when the workspace default is off. Omit to inherit the workspace setting.
     */
    update_lockfiles?: boolean;
    /**
     * Snake_case form of `versionSource`, both spellings are accepted. Per-package override for workspace.versionSource. Lets one package in a monorepo pick its own tag-vs-file resolution without changing the policy for the others. Omit to inherit the workspace setting.
     */
    version_source?: 'highest' | 'tag' | 'file';
  }[];
}
/**
 * Shell commands executed at lifecycle points during release.
 */
export interface Hooks {
  /**
   * Run after bump calculation, before writing version files.
   */
  preBump?: string | HookFunction;
  /**
   * Run after writing version files and generating the changelog. The generated section is in FERRFLOW_CHANGELOG, and anything the hook writes is staged into the release commit.
   */
  postBump?: string | HookFunction;
  /**
   * Run after changelog generation, before git commit.
   */
  preCommit?: string | HookFunction;
  /**
   * Run after the release commit is created, before tagging.
   */
  postCommit?: string | HookFunction;
  /**
   * Run after the release commit, immediately before git tag.
   */
  preTag?: string | HookFunction;
  /**
   * Run after tags are created, before push.
   */
  postTag?: string | HookFunction;
  /**
   * Run after commit and tag, before push.
   */
  prePublish?: string | HookFunction;
  /**
   * Run after push and release creation.
   */
  postPublish?: string | HookFunction;
  /**
   * Run after the release pull request is opened (PR mode), before merge.
   */
  preRelease?: string | HookFunction;
  /**
   * Run once after the release completes successfully.
   */
  onSuccess?: string | HookFunction;
  /**
   * Run once when the release fails; receives FERRFLOW_ERROR_CODE.
   */
  onError?: string | HookFunction;
  /**
   * Behavior when a hook exits non-zero.
   */
  onFailure?: 'abort' | 'continue';
  /**
   * Snake_case form of `preBump` — both spellings are accepted. Run after bump calculation, before writing version files.
   */
  pre_bump?: string | HookFunction;
  /**
   * Snake_case form of `postBump` — both spellings are accepted. Run after writing version files and generating the changelog. The generated section is in FERRFLOW_CHANGELOG, and anything the hook writes is staged into the release commit.
   */
  post_bump?: string | HookFunction;
  /**
   * Snake_case form of `preCommit` — both spellings are accepted. Run after changelog generation, before git commit.
   */
  pre_commit?: string | HookFunction;
  /**
   * Snake_case form of `postCommit` — both spellings are accepted. Run after the release commit is created, before tagging.
   */
  post_commit?: string | HookFunction;
  /**
   * Snake_case form of `preTag` — both spellings are accepted. Run after the release commit, immediately before git tag.
   */
  pre_tag?: string | HookFunction;
  /**
   * Snake_case form of `postTag` — both spellings are accepted. Run after tags are created, before push.
   */
  post_tag?: string | HookFunction;
  /**
   * Snake_case form of `prePublish` — both spellings are accepted. Run after commit and tag, before push.
   */
  pre_publish?: string | HookFunction;
  /**
   * Snake_case form of `postPublish` — both spellings are accepted. Run after push and release creation.
   */
  post_publish?: string | HookFunction;
  /**
   * Snake_case form of `preRelease` — both spellings are accepted. Run after the release pull request is opened (PR mode), before merge.
   */
  pre_release?: string | HookFunction;
  /**
   * Snake_case form of `onSuccess` — both spellings are accepted. Run once after the release completes successfully.
   */
  on_success?: string | HookFunction;
  /**
   * Snake_case form of `onError` — both spellings are accepted. Run once when the release fails; receives FERRFLOW_ERROR_CODE.
   */
  on_error?: string | HookFunction;
  /**
   * Snake_case form of `onFailure` — both spellings are accepted. Behavior when a hook exits non-zero.
   */
  on_failure?: 'abort' | 'continue';
  /**
   * Seconds a hook may run before FerrFlow kills it and its child processes, then applies onFailure. Defaults to 900 (15 minutes).
   */
  timeout?: number;
}
