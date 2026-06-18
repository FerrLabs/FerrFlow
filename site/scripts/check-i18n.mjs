#!/usr/bin/env node
import { readFile, readdir, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = fileURLToPath(new URL('.', import.meta.url));
const SITE_ROOT = join(SCRIPT_DIR, '..');
const SRC_DIR = join(SITE_ROOT, 'src');
const TRANSLATIONS_FILE = join(SRC_DIR, 'app', 'i18n', 'translations.ts');

const SOURCE_EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.jsx', '.astro']);

const T_CALL_RE = /\bt\(\s*[a-zA-Z_$][\w$]*\s*,\s*(['"`])((?:[^\\]|\\.)*?)\1/g;

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

function findBlockEnd(text, openIdx) {
  let depth = 0;
  for (let i = openIdx; i < text.length; i++) {
    const c = text[i];
    if (c === '{') depth++;
    else if (c === '}') {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

function extractLangBlock(source, lang) {
  const re = new RegExp(`\\b${lang}\\s*:\\s*\\{`, 'g');
  const m = re.exec(source);
  if (!m) return null;
  const openIdx = m.index + m[0].length - 1;
  const endIdx = findBlockEnd(source, openIdx);
  if (endIdx < 0) return null;
  return source.slice(openIdx + 1, endIdx);
}

function extractKeys(block) {
  const keys = new Set();
  let i = 0;
  let expectKey = true;
  while (i < block.length) {
    const c = block[i];
    if (c === '/' && block[i + 1] === '/') {
      const nl = block.indexOf('\n', i);
      i = nl < 0 ? block.length : nl + 1;
      continue;
    }
    if (c === '/' && block[i + 1] === '*') {
      const end = block.indexOf('*/', i + 2);
      i = end < 0 ? block.length : end + 2;
      continue;
    }
    if (expectKey && (c === "'" || c === '"' || c === '`')) {
      const quote = c;
      let j = i + 1;
      let key = '';
      while (j < block.length) {
        const ch = block[j];
        if (ch === '\\') {
          key += block[j + 1] ?? '';
          j += 2;
          continue;
        }
        if (ch === quote) break;
        key += ch;
        j++;
      }
      if (j >= block.length) break;
      let k = j + 1;
      while (k < block.length && /\s/.test(block[k])) k++;
      if (block[k] === ':') {
        keys.add(key);
        i = k + 1;
        expectKey = false;
        continue;
      }
      i = j + 1;
      continue;
    }
    if (c === ',') {
      expectKey = true;
      i++;
      continue;
    }
    if (c === "'" || c === '"' || c === '`') {
      const quote = c;
      let j = i + 1;
      while (j < block.length) {
        const ch = block[j];
        if (ch === '\\') {
          j += 2;
          continue;
        }
        if (ch === quote) {
          j++;
          break;
        }
        j++;
      }
      i = j;
      continue;
    }
    i++;
  }
  return keys;
}

async function loadBundles() {
  const raw = await readFile(TRANSLATIONS_FILE, 'utf8');
  const enBlock = extractLangBlock(raw, 'en');
  const frBlock = extractLangBlock(raw, 'fr');
  if (!enBlock) throw new Error(`Could not locate 'en:' block in ${TRANSLATIONS_FILE}`);
  if (!frBlock) throw new Error(`Could not locate 'fr:' block in ${TRANSLATIONS_FILE}`);
  return { en: extractKeys(enBlock), fr: extractKeys(frBlock) };
}

async function extractUsedKeys(srcDir) {
  const files = await walk(srcDir);
  const keys = new Set();
  for (const file of files) {
    const text = await readFile(file, 'utf8');
    let m;
    T_CALL_RE.lastIndex = 0;
    while ((m = T_CALL_RE.exec(text)) !== null) {
      const key = m[2];
      if (!key || /[${}]/.test(key)) continue;
      keys.add(key);
    }
  }
  return keys;
}

function diff(a, b) {
  const out = [];
  for (const x of a) if (!b.has(x)) out.push(x);
  return out.sort();
}

async function pathExists(p) {
  try {
    await stat(p);
    return true;
  } catch {
    return false;
  }
}

let hadError = false;

if (!(await pathExists(TRANSLATIONS_FILE))) {
  console.error(`[i18n] FAIL — translations file not found at ${TRANSLATIONS_FILE}`);
  process.exit(1);
}

const { en, fr } = await loadBundles();

console.log('[i18n] site');

const onlyInFr = diff(fr, en);
const onlyInEn = diff(en, fr);
if (onlyInFr.length || onlyInEn.length) {
  hadError = true;
  if (onlyInFr.length) {
    console.error(`  x ${onlyInFr.length} key(s) in fr but missing in en:`);
    for (const k of onlyInFr.slice(0, 30)) console.error(`      - ${k}`);
    if (onlyInFr.length > 30) console.error(`      ... (+${onlyInFr.length - 30} more)`);
  }
  if (onlyInEn.length) {
    console.error(`  x ${onlyInEn.length} key(s) in en but missing in fr:`);
    for (const k of onlyInEn.slice(0, 30)) console.error(`      - ${k}`);
    if (onlyInEn.length > 30) console.error(`      ... (+${onlyInEn.length - 30} more)`);
  }
} else {
  console.log(`  ok fr / en parity (${en.size} keys)`);
}

const used = await extractUsedKeys(SRC_DIR);
const missing = [];
for (const k of used) {
  if (!fr.has(k) && !en.has(k)) missing.push(k);
}
missing.sort();
if (missing.length) {
  hadError = true;
  console.error(`  x ${missing.length} t() key(s) used in code but absent from both bundles:`);
  for (const k of missing.slice(0, 30)) console.error(`      - ${k}`);
  if (missing.length > 30) console.error(`      ... (+${missing.length - 30} more)`);
} else if (used.size > 0) {
  console.log(`  ok ${used.size} t() call(s) all resolve to a bundle key`);
}

if (hadError) {
  console.error('\n[i18n] FAIL - fix the issues above before merging.');
  process.exit(1);
}
console.log('\n[i18n] OK');
