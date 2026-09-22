import {
  cpSync,
  existsSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const PACKAGE = '@ferrflow/doc';

const DATA_FILES = [
  ['data/benchmarks.json', 'src/app/data/benchmarks.json'],
  ['data/schema', 'public/schema'],
];

const siteRoot = fileURLToPath(new URL('..', import.meta.url));
const contentDir = join(siteRoot, 'src/content');

function fail(message) {
  console.error(`[docs] ${message}`);
  console.error(`[docs] ferrflow.com renders its documentation from ${PACKAGE}.`);
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
  const require = createRequire(import.meta.url);
  const packageRoot = dirname(require.resolve(`${PACKAGE}/package.json`));
  const { version } = require(`${PACKAGE}/package.json`);

  const versions = readdirSync(packageRoot).filter((name) => name.startsWith('docs-'));
  if (versions.length === 0) {
    throw new Error(`${PACKAGE}@${version} carries no docs-* directory`);
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

  // The benchmark numbers and the JSON schema ship in the same package as the
  // pages, so one dependency bump moves all three together. Before this they
  // arrived from a six-hourly job that pushed with an org-admin PAT, and the
  // three could name three different versions of the CLI at once.
  for (const [from, to] of DATA_FILES) {
    const source = join(packageRoot, from);
    if (!existsSync(source)) {
      throw new Error(`${PACKAGE}@${version} carries no ${from}`);
    }
    cpSync(source, join(siteRoot, to), { recursive: true });
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

  const pages = versions.reduce((total, name) => total + countPages(join(packageRoot, name)), 0);
  console.log(
    `[docs] ${pages} pages from ${PACKAGE}@${version} across ${versions.length} versions`,
  );
  console.log(`[docs] benchmarks and schema from ${PACKAGE}@${version}`);
} catch (error) {
  fail(error.message);
}

function countPages(dir) {
  return readdirSync(dir, { withFileTypes: true }).reduce((total, entry) => {
    if (entry.isDirectory()) {
      return total + countPages(join(dir, entry.name));
    }
    return entry.name.endsWith('.md') ? total + 1 : total;
  }, 0);
}
