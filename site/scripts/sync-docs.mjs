import { cpSync, readdirSync, renameSync, rmSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const PACKAGE = '@ferrflow/doc';

const siteRoot = fileURLToPath(new URL('..', import.meta.url));
const contentDir = join(siteRoot, 'src/content');

function fail(message) {
  console.error(`[docs] ${message}`);
  console.error(`[docs] ferrflow.com renders its documentation from ${PACKAGE}.`);
  console.error('[docs] Refusing to build, because an empty content directory');
  console.error('[docs] would publish a site with no documentation and no error.');
  process.exitCode = 1;
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

  const pages = versions.reduce((total, name) => total + countPages(join(packageRoot, name)), 0);
  console.log(
    `[docs] ${pages} pages from ${PACKAGE}@${version} across ${versions.length} versions`,
  );
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
