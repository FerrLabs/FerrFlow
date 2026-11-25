import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';
import {
  cardTitle,
  imagePathFor,
  planImages,
  readPageMeta,
  withImage,
  type Page,
  type PageMeta,
} from './og-pages.ts';

const html = (title: string, description: string, canonical = 'https://example.test/') => `<head>
<meta property="og:image" content="https://ferrflow.com/og.png">
<meta name="twitter:image" content="https://ferrflow.com/og.png">
<meta property="og:title" content="${title}">
<meta property="og:description" content="${description}">
<link rel="canonical" href="${canonical}">
</head>`;

const meta = (title: string, canonicalPath?: string): PageMeta => ({
  title,
  description: `About ${title}`,
  canonicalPath,
});

describe('readPageMeta', () => {
  it('decodes HTML entities in title and description', () => {
    const parsed = readPageMeta(
      html('Widgets &amp; gadgets &#39;ok&#39; &quot;x&quot;', 'a &lt; b &#x2014; c'),
    );
    assert.equal(parsed?.title, `Widgets & gadgets 'ok' "x"`);
    assert.equal(parsed?.description, 'a < b — c');
  });

  it('leaves unknown entities untouched', () => {
    assert.equal(readPageMeta(html('A &nbsp; B', ''))?.title, 'A &nbsp; B');
  });

  it('returns undefined when og:title is missing or empty', () => {
    assert.equal(readPageMeta('<head><title>Plain</title></head>'), undefined);
    assert.equal(readPageMeta(html('', 'desc')), undefined);
  });

  it('defaults a missing description to an empty string', () => {
    const noDescription = '<meta property="og:title" content="Only a title">';
    assert.equal(readPageMeta(noDescription)?.description, '');
  });

  it('keeps the canonical path only for the site origin', () => {
    assert.equal(
      readPageMeta(html('T', 'D', 'https://ferrflow.com/docs/start/'))?.canonicalPath,
      '/docs/start/',
    );
    assert.equal(
      readPageMeta(html('T', 'D', 'https://elsewhere.test/x/'))?.canonicalPath,
      undefined,
    );
    assert.equal(readPageMeta('<meta property="og:title" content="T">')?.canonicalPath, undefined);
  });
});

describe('cardTitle', () => {
  it('strips the docs and changelog suffixes in both languages', () => {
    assert.equal(cardTitle('Quick start: FerrFlow Docs'), 'Quick start');
    assert.equal(cardTitle('Release notes | FerrFlow Changelog'), 'Release notes');
    assert.equal(cardTitle('Notes de version | Changelog FerrFlow'), 'Notes de version');
  });

  it('strips the product prefix and the bare product suffix', () => {
    assert.equal(
      cardTitle('FerrFlow: Entry with a: colon | FerrFlow Changelog'),
      'Entry with a: colon',
    );
    assert.equal(cardTitle('Pricing: FerrFlow'), 'Pricing');
  });

  it('does not strip FerrFlow from the middle of a title', () => {
    assert.equal(cardTitle('Why FerrFlow works: FerrFlow Docs'), 'Why FerrFlow works');
  });

  it('falls back to the full title when stripping would leave nothing', () => {
    assert.equal(cardTitle('FerrFlow: '), 'FerrFlow: ');
  });
});

describe('imagePathFor', () => {
  it('maps pages to png paths under /og', () => {
    assert.equal(imagePathFor('/'), '/og/index.png');
    assert.equal(imagePathFor('/docs/quickstart/'), '/og/docs/quickstart.png');
    assert.equal(
      imagePathFor('/fr/changelog/2026-01-01-entry/'),
      '/og/fr/changelog/2026-01-01-entry.png',
    );
  });
});

describe('withImage', () => {
  it('rewrites og:image and twitter:image to the absolute url', () => {
    const out = withImage(html('T', 'D'), '/og/docs/start.png');
    assert.match(
      out,
      /<meta property="og:image" content="https:\/\/ferrflow\.com\/og\/docs\/start\.png">/,
    );
    assert.match(
      out,
      /<meta name="twitter:image" content="https:\/\/ferrflow\.com\/og\/docs\/start\.png">/,
    );
    assert.doesNotMatch(out, /og\.png/);
  });

  it('leaves the rest of the document untouched', () => {
    const source = html('T', 'D');
    const out = withImage(source, '/og/x.png');
    assert.equal(
      out.replaceAll('https://ferrflow.com/og/x.png', 'https://ferrflow.com/og.png'),
      source,
    );
  });

  it('returns a page without image tags unchanged', () => {
    const bare = '<meta property="og:title" content="T">';
    assert.equal(withImage(bare, '/og/x.png'), bare);
  });
});

describe('planImages', () => {
  const pages: Page[] = [
    { path: '/docs/start/', meta: meta('Start', '/docs/start/') },
    { path: '/v1/docs/start/', meta: meta('Start old', '/docs/start/') },
    { path: '/fr/docs/start/', meta: meta('Demarrage', '/fr/docs/start/') },
    { path: '/404/', meta: meta('404: FerrFlow', '/404/') },
    { path: '/fr/404/', meta: meta('404: FerrFlow') },
    { path: '/en/404/', meta: meta('404: FerrFlow') },
    { path: '/no-meta/', meta: undefined },
  ];

  it('renders one card per canonical page and shares it with duplicates', () => {
    const { jobs, imageByPage } = planImages(pages);
    assert.deepEqual(jobs.map((job) => job.imagePath).sort(), [
      '/og/docs/start.png',
      '/og/fr/docs/start.png',
    ]);
    assert.equal(imageByPage.get('/v1/docs/start/'), '/og/docs/start.png');
    assert.equal(imageByPage.get('/docs/start/'), '/og/docs/start.png');
    assert.equal(imageByPage.get('/fr/docs/start/'), '/og/fr/docs/start.png');
  });

  it('draws the shared card from the canonical page, not the duplicate', () => {
    const job = planImages(pages).jobs.find((j) => j.imagePath === '/og/docs/start.png');
    assert.deepEqual(job, {
      imagePath: '/og/docs/start.png',
      title: 'Start',
      description: 'About Start',
      path: '/docs/start/',
    });
  });

  it('keeps the default image for 404 pages and pages without metadata', () => {
    const { imageByPage } = planImages(pages);
    for (const path of ['/404/', '/fr/404/', '/en/404/', '/no-meta/']) {
      assert.equal(imageByPage.has(path), false, path);
    }
  });

  it('renders its own card when the canonical target has no usable page', () => {
    const orphans: Page[] = [
      { path: '/old/', meta: meta('Old', '/gone/') },
      { path: '/other/', meta: meta('Other', '/404/') },
      { path: '/404/', meta: meta('404: FerrFlow') },
    ];
    const { imageByPage } = planImages(orphans);
    assert.equal(imageByPage.get('/old/'), '/og/old.png');
    assert.equal(imageByPage.get('/other/'), '/og/other.png');
  });

  it('renders a page without a canonical on its own path', () => {
    const { imageByPage, jobs } = planImages([{ path: '/', meta: meta('Home') }]);
    assert.equal(imageByPage.get('/'), '/og/index.png');
    assert.equal(jobs.length, 1);
  });
});
