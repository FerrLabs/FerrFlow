import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';

const SOURCE_EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.jsx', '.html']);
const ID_RE = /@@([A-Za-z0-9_.-]+)/g;

export function getExt(name) {
  const i = name.lastIndexOf('.');
  return i < 0 ? '' : name.slice(i);
}

export async function walk(dir) {
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

export async function extractUsedIds(srcDir) {
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

export function sortedDifference(left, right) {
  return [...left].filter((id) => !right.has(id)).sort((a, b) => a.localeCompare(b));
}
