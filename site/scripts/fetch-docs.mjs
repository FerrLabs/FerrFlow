import { execFileSync } from 'node:child_process';
import {
  cpSync,
  existsSync,
  mkdtempSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = 'FerrLabs/FerrFlow-Docs';
const ref = process.env.FERRFLOW_DOCS_REF || 'main';
const tarball = `https://codeload.github.com/${REPO}/tar.gz/${ref}`;

const siteRoot = fileURLToPath(new URL('..', import.meta.url));
const contentDir = join(siteRoot, 'src/content');

function fail(message) {
  throw new Error(message);
}

const staging = mkdtempSync(join(tmpdir(), 'ferrflow-docs-'));

try {
  const response = await fetch(tarball, { redirect: 'follow' });
  if (!response.ok) {
    fail(`${tarball} returned ${response.status} ${response.statusText}`);
  }

  writeFileSync(join(staging, 'docs.tar.gz'), Buffer.from(await response.arrayBuffer()));

  const extracted = join(staging, 'extracted');
  execFileSync('tar', ['-xzf', 'docs.tar.gz'], { cwd: staging });

  const [topLevel] = readdirSync(staging).filter((name) =>
    existsSync(join(staging, name, 'content')),
  );
  if (!topLevel) {
    fail('the archive has no content/ directory');
  }
  cpSync(join(staging, topLevel, 'content'), extracted, { recursive: true });

  const fetched = readdirSync(extracted).filter((name) => name.startsWith('docs-'));
  if (fetched.length === 0) {
    fail('the archive carries no docs-* directory');
  }

  // Copy everything in beside the live directories first, so the slow half
  // cannot fail with the old docs already deleted. Only once every version is
  // staged on the target disk does the swap run, and a rename is short enough
  // that a failure there leaves one directory missing rather than all of them.
  for (const name of fetched) {
    const incoming = join(contentDir, `${name}.incoming`);
    rmSync(incoming, { recursive: true, force: true });
    cpSync(join(extracted, name), incoming, { recursive: true });
  }
  const live = readdirSync(contentDir).filter(
    (name) => name.startsWith('docs-') && !name.endsWith('.incoming'),
  );
  for (const name of live) {
    rmSync(join(contentDir, name), { recursive: true, force: true });
  }
  for (const name of fetched) {
    renameSync(join(contentDir, `${name}.incoming`), join(contentDir, name));
  }

  const pages = fetched.reduce((total, name) => total + countPages(join(extracted, name)), 0);
  console.log(`[docs] ${pages} pages from ${REPO}@${ref} across ${fetched.length} versions`);
} catch (error) {
  console.error(`[docs] ${error.message}`);
  console.error(`[docs] ferrflow.com renders its documentation from ${REPO}.`);
  console.error('[docs] Refusing to build, because an empty content directory');
  console.error('[docs] would publish a site with no documentation and no error.');
  process.exitCode = 1;
} finally {
  rmSync(staging, { recursive: true, force: true });
}

function countPages(dir) {
  return readdirSync(dir, { withFileTypes: true }).reduce((total, entry) => {
    if (entry.isDirectory()) {
      return total + countPages(join(dir, entry.name));
    }
    return entry.name.endsWith('.md') ? total + 1 : total;
  }, 0);
}
