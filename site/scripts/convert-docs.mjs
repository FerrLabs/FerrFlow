import { readdirSync, readFileSync, mkdirSync, writeFileSync, statSync } from 'node:fs';
import { join, relative, dirname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { convert } from './docs-convert.mjs';

const SITE = fileURLToPath(new URL('../../site/src/content/docs', import.meta.url));
const OUT = fileURLToPath(new URL('../src/content', import.meta.url));

const SOURCES = [
  { docsDir: join(SITE, 'docs'), outDir: join(OUT, 'docs-en') },
  { docsDir: join(SITE, 'fr', 'docs'), outDir: join(OUT, 'docs-fr') },
  { docsDir: join(SITE, 'v0', 'docs'), outDir: join(OUT, 'docs-v0') },
  { docsDir: join(SITE, 'v1', 'docs'), outDir: join(OUT, 'docs-v1') },
  { docsDir: join(SITE, 'v2', 'docs'), outDir: join(OUT, 'docs-v2') },
  { docsDir: join(SITE, 'v3', 'docs'), outDir: join(OUT, 'docs-v3') },
  { docsDir: join(SITE, 'v4', 'docs'), outDir: join(OUT, 'docs-v4') },
  { docsDir: join(SITE, 'fr', 'v4', 'docs'), outDir: join(OUT, 'docs-fr-v4') },
];

function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) out.push(...walk(full));
    else if (entry.endsWith('.mdx')) out.push(full);
  }
  return out;
}

let total = 0;
const generated = [];
for (const { docsDir, outDir } of SOURCES) {
  for (const file of walk(docsDir)) {
    const raw = readFileSync(file, 'utf8');
    const converted = convert(raw);
    const slug = relative(docsDir, file).replace(/\.mdx$/, '.md');
    const target = join(outDir, slug);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, converted, 'utf8');
    generated.push(relative(OUT, target).split(sep).join('/'));
    total++;
  }
}

generated.sort((a, b) => a.localeCompare(b));
for (const g of generated) console.log(g);
console.log(
  `\nConverted ${total} files (${SOURCES.map((s) => relative(OUT, s.outDir)).join(' + ')}).`,
);
