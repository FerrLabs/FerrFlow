import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, it } from 'node:test';
import {
  countPages,
  dropSlugOverride,
  keepPreBlocksWhole,
  slashInternalLinks,
} from './docs-transforms.mjs';

let dir;

beforeEach(async () => {
  dir = await mkdtemp(join(tmpdir(), 'docs-transforms-'));
});

afterEach(() => rm(dir, { recursive: true, force: true }));

const write = async (name, text) => {
  const file = join(dir, name);
  await mkdir(join(file, '..'), { recursive: true });
  await writeFile(file, text);
  return file;
};

describe('slashInternalLinks', () => {
  it('adds the trailing slash to markdown and href links, keeping query and hash', async () => {
    const file = await write(
      'a.md',
      '[x](/docs/intro) <a href="/docs/cfg#top">y</a> [z](/docs/q?a=1)',
    );
    assert.equal(slashInternalLinks(dir), 3);
    assert.equal(
      await readFile(file, 'utf8'),
      '[x](/docs/intro/) <a href="/docs/cfg/#top">y</a> [z](/docs/q/?a=1)',
    );
  });

  it('leaves slashed paths, file paths, external and protocol-relative links alone', async () => {
    const text =
      '[a](/docs/) [b](/schema/config.json) [c](https://x.test/p) [d](//cdn.test/p) [e](rel/p)';
    const file = await write('a.md', text);
    assert.equal(slashInternalLinks(dir), 0);
    assert.equal(await readFile(file, 'utf8'), text);
  });

  it('recurses into directories and ignores non-markdown files', async () => {
    await write('deep/b.md', '[x](/a)');
    const other = await write('c.txt', '[x](/a)');
    assert.equal(slashInternalLinks(dir), 1);
    assert.equal(await readFile(other, 'utf8'), '[x](/a)');
  });
});

describe('keepPreBlocksWhole', () => {
  it('fills blank lines inside raw pre blocks so markdown does not split them', async () => {
    const file = await write('a.md', 'before\n\n<pre>\none\n\ntwo\n</pre>\n\nafter\n');
    assert.equal(keepPreBlocksWhole(dir), 1);
    assert.equal(
      await readFile(file, 'utf8'),
      'before\n\n<pre>\none\n<span></span>\ntwo\n</pre>\n\nafter\n',
    );
  });

  it('does not count or rewrite a pre block without blank lines', async () => {
    const text = '<pre>\none\ntwo\n</pre>\n';
    const file = await write('a.md', text);
    assert.equal(keepPreBlocksWhole(dir), 0);
    assert.equal(await readFile(file, 'utf8'), text);
  });

  it('leaves blank lines outside pre blocks untouched', async () => {
    const text = 'a\n\nb\n';
    await write('a.md', text);
    assert.equal(keepPreBlocksWhole(dir), 0);
  });
});

describe('dropSlugOverride', () => {
  it('removes the slug line from the frontmatter only', async () => {
    const file = await write(
      'a.md',
      '---\ntitle: T\nslug: custom\ndescription: D\n---\nslug: stays in the body\n',
    );
    assert.equal(dropSlugOverride(dir), 1);
    assert.equal(
      await readFile(file, 'utf8'),
      '---\ntitle: T\ndescription: D\n---\nslug: stays in the body\n',
    );
  });

  it('skips pages without frontmatter or without a slug line', async () => {
    await write('plain.md', 'no frontmatter\nslug: x\n');
    await write('noslug.md', '---\ntitle: T\n---\nbody\n');
    assert.equal(dropSlugOverride(dir), 0);
  });
});

describe('countPages', () => {
  it('counts markdown files recursively', async () => {
    await write('a.md', '');
    await write('sub/b.md', '');
    await write('sub/c.txt', '');
    assert.equal(countPages(dir), 2);
  });
});
