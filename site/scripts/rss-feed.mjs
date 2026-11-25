import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import fm from 'front-matter';

export const SITE = 'https://ferrflow.com';

export const esc = (s) =>
  String(s)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');

export function entries(contentDir) {
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

export function feed(items, base) {
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
