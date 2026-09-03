import { execFileSync } from 'node:child_process';
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = 'FerrLabs/Changelog';
const PRODUCT = 'ferrflow';
const ref = process.env.FERRLABS_CHANGELOG_REF || 'main';
const tarball = `https://codeload.github.com/${REPO}/tar.gz/${ref}`;

const siteRoot = fileURLToPath(new URL('..', import.meta.url));
const contentDir = join(siteRoot, 'src/content');
const target = join(contentDir, 'changelog');
const incoming = join(contentDir, 'changelog.incoming');

const PRODUCT_FIELD = /^product:\s*['"]?([A-Za-z0-9_-]+)['"]?\s*$/m;

const staging = mkdtempSync(join(tmpdir(), 'ferrlabs-changelog-'));

try {
  const response = await fetch(tarball, { redirect: 'follow' });
  if (!response.ok) {
    throw new Error(`${tarball} returned ${response.status} ${response.statusText}`);
  }

  writeFileSync(join(staging, 'changelog.tar.gz'), Buffer.from(await response.arrayBuffer()));
  execFileSync('tar', ['-xzf', 'changelog.tar.gz'], { cwd: staging });

  const [topLevel] = readdirSync(staging).filter((name) =>
    existsSync(join(staging, name, 'entries')),
  );
  if (!topLevel) {
    throw new Error('the archive has no entries/ directory');
  }

  const entriesDir = join(staging, topLevel, 'entries');
  const mine = readdirSync(entriesDir)
    .filter((name) => name.endsWith('.md'))
    .filter(
      (name) => PRODUCT_FIELD.exec(readFileSync(join(entriesDir, name), 'utf8'))?.[1] === PRODUCT,
    );

  if (mine.length === 0) {
    throw new Error(`the archive carries no entry with product: ${PRODUCT}`);
  }

  // Stage the whole set beside the live directory, so the half that can fail
  // never runs with the existing entries already deleted.
  rmSync(incoming, { recursive: true, force: true });
  mkdirSync(incoming, { recursive: true });
  for (const name of mine) {
    copyFileSync(join(entriesDir, name), join(incoming, name));
  }

  rmSync(target, { recursive: true, force: true });
  renameSync(incoming, target);

  console.log(`[changelog] ${mine.length} ${PRODUCT} entries from ${REPO}@${ref}`);
} catch (error) {
  console.error(`[changelog] ${error.message}`);
  console.error(`[changelog] This site renders its changelog from ${REPO}.`);
  console.error('[changelog] Refusing to build, because an empty content directory');
  console.error('[changelog] would publish a changelog that silently lost entries.');
  process.exitCode = 1;
} finally {
  rmSync(staging, { recursive: true, force: true });
}
