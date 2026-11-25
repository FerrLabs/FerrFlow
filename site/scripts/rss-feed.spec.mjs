import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { entries, esc, feed } from './rss-feed.mjs';

const entry = (fields) =>
  `---\n${Object.entries(fields)
    .map(([k, v]) => `${k}: ${JSON.stringify(v)}`)
    .join('\n')}\n---\nbody\n`;

let dir;

before(async () => {
  dir = await mkdtemp(join(tmpdir(), 'rss-feed-'));
  const base = { product: 'ferrflow', type: 'feature', summary: 's' };
  await writeFile(join(dir, 'older.md'), entry({ ...base, title: 'Older', date: '2026-01-02' }));
  await writeFile(join(dir, 'newer.md'), entry({ ...base, title: 'Newer', date: '2026-03-04' }));
  await writeFile(
    join(dir, 'draft.md'),
    entry({ ...base, title: 'Draft', date: '2026-05-06', draft: true }),
  );
  await writeFile(
    join(dir, 'other.md'),
    entry({ ...base, title: 'Other', date: '2026-05-06', product: 'sidekick' }),
  );
  await writeFile(join(dir, 'readme.txt'), 'not markdown');
});

after(() => rm(dir, { recursive: true, force: true }));

describe('esc', () => {
  it('escapes the four XML-significant characters and nothing else', () => {
    assert.equal(esc(`a & b < c > d "e" 'f'`), `a &amp; b &lt; c &gt; d &quot;e&quot; 'f'`);
  });

  it('escapes the ampersand first so entities are not double-escaped', () => {
    assert.equal(esc('<'), '&lt;');
  });

  it('coerces non-strings', () => {
    assert.equal(esc(42), '42');
  });
});

describe('entries', () => {
  it('keeps published ferrflow entries, newest first, with their slug', () => {
    const list = entries(dir);
    assert.deepEqual(
      list.map((e) => e.slug),
      ['newer', 'older'],
    );
    assert.equal(list[0].title, 'Newer');
  });

  it('returns an empty list when the directory is missing', () => {
    assert.deepEqual(entries(join(dir, 'nope')), []);
  });
});

describe('feed', () => {
  const items = [
    {
      slug: 'one',
      title: 'Fish & <chips>',
      summary: 'a "quoted" summary',
      product: 'ferrflow',
      type: 'fix',
      date: '2026-03-04',
    },
  ];

  it('builds an RSS document with escaped item fields and absolute links', () => {
    const xml = feed(items, '/changelog');
    assert.match(xml, /<title>Fish &amp; &lt;chips&gt;<\/title>/);
    assert.match(xml, /<description>a &quot;quoted&quot; summary<\/description>/);
    assert.match(xml, /<link>https:\/\/ferrflow\.com\/changelog\/one\/<\/link>/);
    assert.match(xml, /<guid isPermaLink="true">https:\/\/ferrflow\.com\/changelog\/one\/<\/guid>/);
    assert.match(xml, /<pubDate>Wed, 04 Mar 2026 00:00:00 GMT<\/pubDate>/);
    assert.match(xml, /<category>fix<\/category>/);
  });

  it('points the self link at the locale base', () => {
    const xml = feed(items, '/fr/changelog');
    assert.match(xml, /<atom:link href="https:\/\/ferrflow\.com\/fr\/changelog\/rss\.xml"/);
  });

  it('emits a valid empty channel when there are no items', () => {
    const xml = feed([], '/changelog');
    assert.doesNotMatch(xml, /<item>/);
    assert.match(xml, /<\/channel>\n<\/rss>\n$/);
  });
});
