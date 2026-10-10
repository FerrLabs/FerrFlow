---
title: 'FerrFlow: Migrating from changesets now finds your packages'
summary: '`ferrflow migrate --from changesets` reads your workspace globs and scaffolds one package entry per workspace package, instead of a single root package you had to expand by hand.'
date: 2026-07-28T14:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/775
docsLink: https://ferrflow.com/docs/reference/cli
---

changesets is a monorepo tool, so migrating from it used to end with the least useful part done by hand: FerrFlow scaffolded a single root package and asked you to list every workspace package yourself. On a repo with thirty packages that's thirty entries to write before anything runs.

The migration now reads the same workspace declaration changesets does: `workspaces` in `package.json` (both the array and the yarn-berry `{ packages: [...] }` form) and `packages:` in `pnpm-workspace.yaml`: expands the globs, and writes one `package` entry per discovered package, each pointing at its own `package.json` and `CHANGELOG.md`.

The practical win is in the version groups. `linked` and `fixed` name packages by npm name, and those names now match packages that actually exist in the generated config, so it passes `ferrflow validate` straight out of the migration. If a group names something outside the workspace, the migration report says which one rather than leaving you to find out at validation time.

Dependency directories are skipped, so a `node_modules` full of manifests is never mistaken for your workspace. A repo without a workspace declaration still gets a single root package, as before.
