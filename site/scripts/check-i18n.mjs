#!/usr/bin/env node
import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = fileURLToPath(new URL('.', import.meta.url));
const SITE_ROOT = join(SCRIPT_DIR, '..');
const SRC_DIR = join(SITE_ROOT, 'src');
const FR_FILE = join(SRC_DIR, 'i18n', 'fr.json');

const SOURCE_EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.jsx', '.html']);
const ID_RE = /@@([A-Za-z0-9_.-]+)/g;

function getExt(name) {
  const i = name.lastIndexOf('.');
  return i < 0 ? '' : name.slice(i);
}

async function walk(dir) {
  const out = [];
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === 'dist' || entry.name.startsWith('.'))
        continue;
      out.push(...(await walk(full)));
    } else if (SOURCE_EXTENSIONS.has(getExt(entry.name))) {
      out.push(full);
    }
  }
  return out;
}

async function extractUsedIds(srcDir) {
  const files = await walk(srcDir);
  const ids = new Set();
  for (const file of files) {
    const text = await readFile(file, 'utf8');
    let m;
    ID_RE.lastIndex = 0;
    while ((m = ID_RE.exec(text)) !== null) ids.add(m[1]);
  }
  return ids;
}

let hadError = false;
console.log('[i18n] site');

const fr = JSON.parse(await readFile(FR_FILE, 'utf8'));
const frKeys = new Set(Object.keys(fr));
const used = await extractUsedIds(SRC_DIR);

const missing = [...used].filter((id) => !frKeys.has(id)).sort();
if (missing.length) {
  hadError = true;
  console.error(`  x ${missing.length} $localize id(s) used in source but missing from fr.json:`);
  for (const k of missing.slice(0, 40)) console.error(`      - ${k}`);
  if (missing.length > 40) console.error(`      ... (+${missing.length - 40} more)`);
} else {
  console.log(`  ok ${used.size} $localize id(s) all have a fr.json translation`);
}

const unused = [...frKeys].filter((id) => !used.has(id)).sort();
if (unused.length) {
  console.warn(
    `  ! ${unused.length} fr.json key(s) not referenced in source (dead): ${unused.join(', ')}`,
  );
}

if (hadError) {
  console.error('\n[i18n] FAIL - add the missing translations to src/i18n/fr.json before merging.');
  process.exit(1);
}
console.log('\n[i18n] OK');
