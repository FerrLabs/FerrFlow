import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { entries, feed } from './rss-feed.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const contentDir = join(root, 'src/content/changelog');
const outDir = join(root, 'dist/analog/public');

const items = entries(contentDir);
mkdirSync(join(outDir, 'changelog'), { recursive: true });
mkdirSync(join(outDir, 'fr', 'changelog'), { recursive: true });
writeFileSync(join(outDir, 'changelog', 'rss.xml'), feed(items, '/changelog'), 'utf8');
writeFileSync(join(outDir, 'fr', 'changelog', 'rss.xml'), feed(items, '/fr/changelog'), 'utf8');
console.log(`[rss] wrote ${items.length} items to /changelog/rss.xml and /fr/changelog/rss.xml`);
