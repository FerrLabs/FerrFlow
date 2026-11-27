import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import {
  convert,
  dedent,
  dropImports,
  findMatching,
  getAttr,
  splitFrontmatter,
} from './docs-convert.mjs';

describe('splitFrontmatter', () => {
  it('separates the leading fence from the body', () => {
    const { frontmatter, body } = splitFrontmatter('---\ntitle: A\n---\nbody\n');
    assert.equal(frontmatter, '---\ntitle: A\n---\n');
    assert.equal(body, 'body\n');
  });

  it('handles CRLF fences', () => {
    const { frontmatter, body } = splitFrontmatter('---\r\ntitle: A\r\n---\r\nbody');
    assert.equal(frontmatter, '---\r\ntitle: A\r\n---\r\n');
    assert.equal(body, 'body');
  });

  it('returns the whole text as body without a closing fence', () => {
    const raw = '---\ntitle: A\nbody';
    assert.deepEqual(splitFrontmatter(raw), { frontmatter: '', body: raw });
  });

  it('returns the whole text as body without an opening fence', () => {
    assert.deepEqual(splitFrontmatter('plain\n---\nx'), { frontmatter: '', body: 'plain\n---\nx' });
  });

  it('stops at the first closing fence', () => {
    const { body } = splitFrontmatter('---\na: 1\n---\nx\n---\ny');
    assert.equal(body, 'x\n---\ny');
  });

  it('finishes quickly on a long unterminated block', () => {
    const raw = `---\n${'a\r\n'.repeat(50000)}`;
    const start = performance.now();
    splitFrontmatter(raw);
    assert.ok(performance.now() - start < 1000);
  });
});

describe('dropImports', () => {
  it('removes import lines and keeps the rest', () => {
    const out = dropImports('import { Card } from \'@lib/ui\';\nkeep me\nimport X from "y"\n');
    assert.equal(out, 'keep me\n');
  });

  it('keeps lines that only mention from or import', () => {
    const body = 'import is a word\nwe copy from "here"\nimport { a } from b\n';
    assert.equal(dropImports(body), body);
  });

  it('handles CRLF line endings', () => {
    assert.equal(dropImports('import X from "y";\r\nkeep\r\n'), 'keep\r\n');
  });
});

describe('dedent', () => {
  it('strips the common indent and the leading newline', () => {
    assert.equal(dedent('\n    a\n      b\n    c'), 'a\n  b\nc');
  });

  it('ignores blank lines when measuring the indent', () => {
    assert.equal(dedent('  a\n\n  b'), 'a\n\nb');
  });
});

describe('getAttr', () => {
  it('reads double-quoted, single-quoted and braced values', () => {
    assert.equal(getAttr(' title="Hello there" x=1', 'title'), 'Hello there');
    assert.equal(getAttr(" title='Hi'", 'title'), 'Hi');
    assert.equal(getAttr(' cols={3}', 'cols'), '3');
  });

  it('returns undefined when the attribute or the whole string is missing', () => {
    assert.equal(getAttr(' a="1"', 'b'), undefined);
    assert.equal(getAttr(undefined, 'b'), undefined);
  });
});

describe('findMatching', () => {
  it('skips nested tags of the same name', () => {
    const body = '<Note>outer <Note>inner</Note> tail</Note> after';
    const start = '<Note>'.length;
    const { innerStart, innerEnd, end } = findMatching(body, 'Note', start);
    assert.equal(body.slice(innerStart, innerEnd), 'outer <Note>inner</Note> tail');
    assert.equal(body.slice(end), ' after');
  });

  it('throws when the tag is never closed', () => {
    assert.throws(() => findMatching('<Note>oops', 'Note', 6), /Unbalanced <Note>/);
  });
});

describe('convert', () => {
  it('keeps the frontmatter and renders an Aside with its type and title', () => {
    const out = convert(
      '---\ntitle: T\n---\n<Aside type="caution" title="Careful">\n  Mind **this**.\n</Aside>\n',
    );
    assert.ok(out.startsWith('---\ntitle: T\n---\n'));
    assert.match(out, /<aside class="ferr-aside ferr-aside--warning">/);
    assert.match(out, /<p class="ferr-aside__title">Careful<\/p>/);
    assert.match(out, /<strong>this<\/strong>/);
  });

  it('falls back to the note style for an unknown aside type', () => {
    assert.match(convert('<Aside type="weird">\nx\n</Aside>'), /ferr-aside--note/);
  });

  it('renders shorthand components with their own style', () => {
    assert.match(convert('<Tip>\nt\n</Tip>'), /ferr-aside--tip/);
    assert.match(convert('<Info>\ni\n</Info>'), /ferr-aside--info/);
    assert.match(convert('<Warning>\nw\n</Warning>'), /ferr-aside--warning/);
    assert.match(convert('<Note title="N">\nn\n</Note>'), /ferr-aside__title">N</);
  });

  it('renders a card group with links and plain cards', () => {
    const out = convert(
      '<CardGroup cols={3}>\n<Card title="Go" href="/go">\nbody\n</Card>\n<Card title="Stay">\nmore\n</Card>\n</CardGroup>',
    );
    assert.match(out, /<div class="ferr-card-group" data-cols="3">/);
    assert.match(out, /<a class="ferr-card" href="\/go">/);
    assert.match(out, /<div class="ferr-card"><p class="ferr-card__title">Stay<\/p>/);
  });

  it('defaults a card group to two columns', () => {
    assert.match(convert('<CardGroup>\n</CardGroup>'), /data-cols="2"/);
  });

  it('renders tab items with their labels', () => {
    const out = convert('<Tabs>\n<TabItem label="npm">\nrun\n</TabItem>\n</Tabs>');
    assert.match(out, /<div class="ferr-tabs">/);
    assert.match(out, /data-label="npm"><p class="ferr-tab__label">npm<\/p>/);
  });

  it('unwraps Steps into plain markdown output', () => {
    const out = convert('<Steps>\n1. one\n2. two\n</Steps>');
    assert.match(out, /<ol>/);
    assert.doesNotMatch(out, /Steps/);
  });

  it('converts :::directive blocks, with and without a title', () => {
    const out = convert(':::tip[Pro tip]\nuse it\n:::\n\n:::danger\nstop\n:::\n');
    assert.match(out, /ferr-aside--tip"><p class="ferr-aside__title">Pro tip<\/p>/);
    assert.match(out, /ferr-aside--warning"><div/);
  });

  it('drops imports and collapses runs of blank lines', () => {
    const out = convert("import A from 'a';\n\n\n\n\ntext\n\n\n\nmore\n");
    assert.equal(out, 'text\n\nmore\n');
  });

  it('handles nested components of different kinds', () => {
    const out = convert('<Tabs>\n<TabItem label="a">\n<Tip>\nx\n</Tip>\n</TabItem>\n</Tabs>');
    assert.match(out, /ferr-tabs/);
    assert.match(out, /ferr-tab__body/);
  });
});
