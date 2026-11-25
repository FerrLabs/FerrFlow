import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { isCanonicalToItself, prerenderedPaths, sitemapPaths, sitemapXml } from './sitemap.mjs';

let out;

const page = async (path, canonical) => {
  const dir = join(out, path);
  await mkdir(dir, { recursive: true });
  const link = canonical === undefined ? '' : `<link rel="canonical" href="${canonical}">`;
  await writeFile(join(dir, 'index.html'), `<html><head>${link}</head></html>`);
};

before(async () => {
  out = await mkdtemp(join(tmpdir(), 'sitemap-'));
  await page('', 'https://ferrflow.com/');
  await page('docs/intro', 'https://ferrflow.com/docs/intro/');
  await page('fr/docs/intro');
  await page('docs/moved', 'https://ferrflow.com/docs/elsewhere/');
  await page('404', 'https://ferrflow.com/404/');
  await page('fr/404');
  await page('en/docs');
  await writeFile(join(out, 'robots.txt'), 'x');
});

after(() => rm(out, { recursive: true, force: true }));

describe('prerenderedPaths', () => {
  it('maps each index.html to its URL path, root included', () => {
    assert.deepEqual(prerenderedPaths(out).sort(), [
      '/',
      '/404/',
      '/docs/intro/',
      '/docs/moved/',
      '/en/docs/',
      '/fr/404/',
      '/fr/docs/intro/',
    ]);
  });
});

describe('isCanonicalToItself', () => {
  it('is true when the canonical matches the path or is absent', () => {
    assert.equal(isCanonicalToItself(out, '/docs/intro/'), true);
    assert.equal(isCanonicalToItself(out, '/fr/docs/intro/'), true);
  });

  it('is false when the canonical points elsewhere', () => {
    assert.equal(isCanonicalToItself(out, '/docs/moved/'), false);
  });
});

describe('sitemapPaths', () => {
  it('drops 404 pages, /en/ aliases and pages canonical to another URL', () => {
    assert.deepEqual(sitemapPaths(out), ['/', '/docs/intro/', '/fr/docs/intro/']);
  });
});

describe('sitemapXml', () => {
  it('lists one url element per path', () => {
    const xml = sitemapXml(['/', '/docs/']);
    assert.match(
      xml,
      /<url><loc>https:\/\/ferrflow\.com\/<\/loc><\/url>\n {2}<url><loc>https:\/\/ferrflow\.com\/docs\/<\/loc><\/url>/,
    );
    assert.match(xml, /^<\?xml version="1.0" encoding="UTF-8"\?>\n<urlset/);
  });
});
