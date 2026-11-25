#!/usr/bin/env node
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { extractUsedIds, sortedDifference } from './i18n-ids.mjs';

const SCRIPT_DIR = fileURLToPath(new URL('.', import.meta.url));
const SITE_ROOT = join(SCRIPT_DIR, '..');
const SRC_DIR = join(SITE_ROOT, 'src');
const FR_FILE = join(SRC_DIR, 'i18n', 'fr.json');

let hadError = false;
console.log('[i18n] site');

const fr = JSON.parse(await readFile(FR_FILE, 'utf8'));
const frKeys = new Set(Object.keys(fr));
const used = await extractUsedIds(SRC_DIR);

const missing = sortedDifference(used, frKeys);
if (missing.length) {
  hadError = true;
  console.error(`  x ${missing.length} $localize id(s) used in source but missing from fr.json:`);
  for (const k of missing.slice(0, 40)) console.error(`      - ${k}`);
  if (missing.length > 40) console.error(`      ... (+${missing.length - 40} more)`);
} else {
  console.log(`  ok ${used.size} $localize id(s) all have a fr.json translation`);
}

const unused = sortedDifference(frKeys, used);
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
