import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { extractUsedIds, getExt, sortedDifference, walk } from './i18n-ids.mjs';

let root;

before(async () => {
  root = await mkdtemp(join(tmpdir(), 'i18n-ids-'));
  await mkdir(join(root, 'pages', 'deep'), { recursive: true });
  await mkdir(join(root, 'node_modules', 'pkg'), { recursive: true });
  await mkdir(join(root, '.hidden'), { recursive: true });
  await mkdir(join(root, 'dist'), { recursive: true });
  await writeFile(join(root, 'a.ts'), '$localize`:@@alpha.title:Hello` $localize`:@@beta-1:x`');
  await writeFile(join(root, 'pages', 'b.html'), '<p i18n="@@pages.b_one">x</p>');
  await writeFile(join(root, 'pages', 'deep', 'c.js'), 'const k = "@@deep.key";');
  await writeFile(join(root, 'pages', 'notes.md'), '@@ignored.markdown');
  await writeFile(join(root, 'node_modules', 'pkg', 'x.ts'), '@@from.node_modules');
  await writeFile(join(root, '.hidden', 'x.ts'), '@@from.hidden');
  await writeFile(join(root, 'dist', 'x.ts'), '@@from.dist');
});

after(() => rm(root, { recursive: true, force: true }));

describe('getExt', () => {
  it('returns the extension with its dot, or an empty string', () => {
    assert.equal(getExt('a.component.ts'), '.ts');
    assert.equal(getExt('Makefile'), '');
  });
});

describe('walk', () => {
  it('lists source files and skips node_modules, dist and dot directories', async () => {
    const files = (await walk(root)).map((f) => f.slice(root.length + 1)).sort();
    assert.deepEqual(files, ['a.ts', join('pages', 'b.html'), join('pages', 'deep', 'c.js')]);
  });

  it('returns nothing for a directory that does not exist', async () => {
    assert.deepEqual(await walk(join(root, 'missing')), []);
  });
});

describe('extractUsedIds', () => {
  it('collects every distinct id from the source files', async () => {
    const ids = await extractUsedIds(root);
    assert.deepEqual([...ids].sort(), ['alpha.title', 'beta-1', 'deep.key', 'pages.b_one']);
  });

  it('counts an id once however often it appears', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'i18n-dup-'));
    await writeFile(join(dir, 'a.ts'), '@@same @@same');
    await writeFile(join(dir, 'b.ts'), '@@same');
    assert.equal((await extractUsedIds(dir)).size, 1);
    await rm(dir, { recursive: true, force: true });
  });
});

describe('sortedDifference', () => {
  it('returns the ids missing from the right side, sorted', () => {
    const left = new Set(['b', 'a', 'c']);
    const right = new Set(['c']);
    assert.deepEqual(sortedDifference(left, right), ['a', 'b']);
  });

  it('is empty when the right side covers the left', () => {
    assert.deepEqual(sortedDifference(new Set(['a']), new Set(['a', 'b'])), []);
  });

  it('sorts locale-aware rather than by code unit', () => {
    assert.deepEqual(sortedDifference(new Set(['b', 'A', 'a']), new Set()), ['a', 'A', 'b']);
  });
});
