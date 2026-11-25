import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';
import { availableSlugs, navForVersion } from './docs-slugs.ts';

const SLUGS = {
  'docs-en': ['intro', 'config'],
  'docs-fr': ['intro'],
  'docs-v3': ['intro'],
  'docs-fr-v4': ['intro', 'config'],
  'docs-v4': ['intro'],
};

const NAV = [
  { label: 'Start', items: [{ label: 'Intro', slug: 'intro' }] },
  { label: 'Ref', items: [{ label: 'Config', slug: 'config' }] },
];

describe('availableSlugs', () => {
  it('uses the english directory for the current docs', () => {
    assert.deepEqual([...availableSlugs(SLUGS, 'current', 'en')], ['intro', 'config']);
  });

  it('prefers the locale directory and falls back to english for current', () => {
    assert.deepEqual([...availableSlugs(SLUGS, 'current', 'fr')], ['intro']);
    assert.deepEqual([...availableSlugs({ 'docs-en': ['x'] }, 'current', 'fr')], ['x']);
  });

  it('falls back to the english snapshot when a version was never translated', () => {
    assert.deepEqual([...availableSlugs(SLUGS, 'v3', 'fr')], ['intro']);
  });

  it('uses the translated snapshot when one exists', () => {
    assert.deepEqual([...availableSlugs(SLUGS, 'v4', 'fr')], ['intro', 'config']);
  });

  it('skips an empty directory and reports nothing for an unknown version', () => {
    assert.deepEqual(
      [...availableSlugs({ 'docs-fr': [], 'docs-en': ['a'] }, 'current', 'fr')],
      ['a'],
    );
    assert.equal(availableSlugs(SLUGS, 'v9', 'en').size, 0);
  });
});

describe('navForVersion', () => {
  it('drops entries without a page and sections left empty', () => {
    const nav = navForVersion(SLUGS, NAV, 'v3', 'en');
    assert.deepEqual(nav, [{ label: 'Start', items: [{ label: 'Intro', slug: 'intro' }] }]);
  });

  it('returns the nav untouched when no slugs are available', () => {
    assert.equal(navForVersion({}, NAV, 'current', 'en'), NAV);
  });
});
