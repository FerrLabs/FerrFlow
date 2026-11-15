import { readdirSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import fm from 'front-matter';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const contentDir = join(root, 'src/content/changelog');
const outDir = join(root, 'dist/analog/public');
const SITE = 'https://ferrflow.com';

const esc = (s) =>
  String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');

function entries() {
  let files;
  try {
    files = readdirSync(contentDir).filter((f) => f.endsWith('.md'));
  } catch {
    return [];
  }
  return files
    .map((file) => {
      const { attributes } = fm(readFileSync(join(contentDir, file), 'utf8'));
      return { slug: file.replace(/\.md$/, ''), ...attributes };
    })
    .filter((e) => !e.draft && e.product === 'ferrflow')
    .sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime());
}

function feed(items, base) {
  const xmlItems = items
    .map((e) => {
      const link = `${SITE}${base}/${e.slug}/`;
      return `    <item>
      <title>${esc(e.title)}</title>
      <link>${link}</link>
      <guid isPermaLink="true">${link}</guid>
      <pubDate>${new Date(e.date).toUTCString()}</pubDate>
      <description>${esc(e.summary)}</description>
      <category>${esc(e.product)}</category>
      <category>${esc(e.type)}</category>
    </item>`;
    })
    .join('\n');

  return `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>FerrFlow Changelog</title>
    <description>Every feature, fix and improvement to FerrFlow.</description>
    <link>${SITE}${base}/</link>
    <atom:link href="${SITE}${base}/rss.xml" rel="self" type="application/rss+xml" />
    <language>en-us</language>
${xmlItems}
  </channel>
</rss>
`;
}

const items = entries();
mkdirSync(join(outDir, 'changelog'), { recursive: true });
mkdirSync(join(outDir, 'fr', 'changelog'), { recursive: true });
writeFileSync(join(outDir, 'changelog', 'rss.xml'), feed(items, '/changelog'), 'utf8');
writeFileSync(join(outDir, 'fr', 'changelog', 'rss.xml'), feed(items, '/fr/changelog'), 'utf8');
console.log(`[rss] wrote ${items.length} items to /changelog/rss.xml and /fr/changelog/rss.xml`);
