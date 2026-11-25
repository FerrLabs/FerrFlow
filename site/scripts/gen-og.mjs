import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { planImages, readPageMeta, withImage } from '../src/og/og-pages.ts';
import { renderCard } from './og-card.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const outDir = join(here, '..', 'dist/analog/public');

function prerenderedPages(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return prerenderedPages(full);
    if (entry.name !== 'index.html') return [];
    const path = relative(outDir, dir).replace(/\\/g, '/');
    return [{ path: path ? `/${path}/` : '/', file: full, html: readFileSync(full, 'utf8') }];
  });
}

async function writeCard(job) {
  const file = join(outDir, job.imagePath);
  mkdirSync(dirname(file), { recursive: true });
  const png = await renderCard(job);
  writeFileSync(file, png);
  return png.length;
}

const pages = prerenderedPages(outDir);
const { jobs, imageByPage } = planImages(
  pages.map(({ path, html }) => ({ path, meta: readPageMeta(html) })),
);

let bytes = 0;
for (const job of jobs) bytes += await writeCard(job);

for (const { path, file, html } of pages) {
  const imagePath = imageByPage.get(path);
  if (imagePath) writeFileSync(file, withImage(html, imagePath), 'utf8');
}

console.log(
  `[og] wrote ${jobs.length} cards (${(bytes / 1024 / 1024).toFixed(1)} MiB) for ${imageByPage.size} of ${pages.length} pages`,
);
