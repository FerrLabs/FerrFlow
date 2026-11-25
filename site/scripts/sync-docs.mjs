import {
  cpSync,
  existsSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  countPages,
  dropSlugOverride,
  keepPreBlocksWhole,
  slashInternalLinks,
} from './docs-transforms.mjs';

const SOURCE = 'docs/site';

const DATA_FILES = [
  ['data/benchmarks.json', 'src/app/data/benchmarks.json'],
  ['data/schema', 'public/schema'],
];

const siteRoot = fileURLToPath(new URL('..', import.meta.url));
const contentDir = join(siteRoot, 'src/content');

function fail(message) {
  console.error(`[docs] ${message}`);
  console.error(`[docs] ferrflow.com renders its documentation from ${SOURCE}.`);
  console.error('[docs] Refusing to build, because an empty content directory');
  console.error('[docs] would publish a site with no documentation and no error.');
  process.exitCode = 1;
}

let previousReleaseSize;
try {
  previousReleaseSize = JSON.parse(
    readFileSync(join(siteRoot, 'src/app/data/benchmarks.json'), 'utf8'),
  ).ferrflow_release_size_mb;
} catch {
  previousReleaseSize = undefined;
}

try {
  const packageRoot = join(siteRoot, '../docs/site');
  const { version } = JSON.parse(readFileSync(join(packageRoot, 'package.json'), 'utf8'));

  const versions = readdirSync(packageRoot).filter((name) => name.startsWith('docs-'));
  if (versions.length === 0) {
    throw new Error(`${SOURCE}@${version} carries no docs-* directory`);
  }

  // Stage beside the live directories first, so the slow half cannot fail with
  // the existing docs already deleted. Only once every version is on the target
  // disk does the swap run, and a rename is short enough that a failure there
  // leaves one directory missing rather than all of them.
  for (const name of versions) {
    const incoming = join(contentDir, `${name}.incoming`);
    rmSync(incoming, { recursive: true, force: true });
    cpSync(join(packageRoot, name), incoming, { recursive: true });
  }
  for (const name of readdirSync(contentDir).filter(
    (name) => name.startsWith('docs-') && !name.endsWith('.incoming'),
  )) {
    rmSync(join(contentDir, name), { recursive: true, force: true });
  }
  for (const name of versions) {
    renameSync(join(contentDir, `${name}.incoming`), join(contentDir, name));
  }

  const slashed = versions.reduce(
    (total, name) => total + slashInternalLinks(join(contentDir, name)),
    0,
  );
  console.log(`[docs] added the trailing slash to ${slashed} internal links`);

  const unslugged = versions.reduce(
    (total, name) => total + dropSlugOverride(join(contentDir, name)),
    0,
  );
  if (unslugged > 0) {
    console.log(`[docs] dropped the frontmatter slug override from ${unslugged} pages`);
  }

  const unbroken = versions.reduce(
    (total, name) => total + keepPreBlocksWhole(join(contentDir, name)),
    0,
  );
  console.log(`[docs] kept ${unbroken} raw <pre> blocks whole across blank lines`);

  // The benchmark numbers and the JSON schema ship in the same package as the
  // pages, so one dependency bump moves all three together. Before this they
  // arrived from a six-hourly job that pushed with an org-admin PAT, and the
  // three could name three different versions of the CLI at once.
  for (const [from, to] of DATA_FILES) {
    const source = join(packageRoot, from);
    if (!existsSync(source)) {
      throw new Error(`${SOURCE}@${version} carries no ${from}`);
    }
    const destination = join(siteRoot, to);
    if (statSync(source).isDirectory()) {
      rmSync(destination, { recursive: true, force: true });
    }
    cpSync(source, destination, { recursive: true });
  }

  // `ferrflow_release_size_mb` is the linux-x64 tarball, and the release that
  // publishes this package cannot know its own asset sizes yet, so the package
  // never carries it. Losing it is not a missing stat: the landing page falls
  // back to `ferrflow_binary_size_mb`, the unstripped CI build, which is 8.0 MB
  // against a real download of 3.4. Carry the last known value forward.
  const benchPath = join(siteRoot, 'src/app/data/benchmarks.json');
  const incoming = JSON.parse(readFileSync(benchPath, 'utf8'));
  if (previousReleaseSize && !incoming.ferrflow_release_size_mb) {
    incoming.ferrflow_release_size_mb = previousReleaseSize;
    writeFileSync(
      benchPath,
      `${JSON.stringify(incoming, null, 2)}
`,
    );
    console.log(`[docs] carried ferrflow_release_size_mb=${previousReleaseSize} forward`);
  } else if (!incoming.ferrflow_release_size_mb) {
    console.warn('[docs] no ferrflow_release_size_mb, in the package or carried forward.');
    console.warn('[docs] The landing page will show the unstripped CI build size instead,');
    console.warn('[docs] which is roughly 2.4x the real download.');
  }

  const changelogSource = join(packageRoot, 'changelog');
  if (!existsSync(changelogSource)) {
    throw new Error(`${SOURCE}@${version} carries no changelog directory`);
  }
  const entries = readdirSync(changelogSource).filter((name) => name.endsWith('.md'));
  if (entries.length === 0) {
    throw new Error(`${SOURCE}@${version} carries no changelog entry`);
  }
  const changelogIncoming = join(contentDir, 'changelog.incoming');
  rmSync(changelogIncoming, { recursive: true, force: true });
  cpSync(changelogSource, changelogIncoming, { recursive: true });
  rmSync(join(contentDir, 'changelog'), { recursive: true, force: true });
  renameSync(changelogIncoming, join(contentDir, 'changelog'));
  console.log(`[docs] ${entries.length} changelog entries from ${SOURCE}@${version}`);

  const pages = versions.reduce((total, name) => total + countPages(join(packageRoot, name)), 0);
  console.log(`[docs] ${pages} pages from ${SOURCE}@${version} across ${versions.length} versions`);
  console.log(`[docs] benchmarks and schema from ${SOURCE}@${version}`);
} catch (error) {
  fail(error.message);
}
