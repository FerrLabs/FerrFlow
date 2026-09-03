import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const contentDir = join(root, 'docs/site');

const FRONTMATTER = /^---\r?\n([\s\S]*?)\r?\n---\r?\n/;
const FIELD = /^([A-Za-z_][A-Za-z0-9_]*):\s*(.*?)\s*$/;

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    return statSync(full).isDirectory() ? walk(full) : [full];
  });
}

function frontmatter(raw) {
  const block = FRONTMATTER.exec(raw);
  if (!block) {
    return null;
  }
  const fields = new Map();
  for (const line of block[1].split(/\r?\n/)) {
    const field = FIELD.exec(line);
    if (field) {
      fields.set(field[1], field[2].replace(/^['"]|['"]$/g, ''));
    }
  }
  return fields;
}

const problems = [];
const versions = readdirSync(contentDir).filter((name) => name.startsWith('docs-'));
const files = versions.flatMap((name) => walk(join(contentDir, name)));

for (const file of files) {
  const where = relative(root, file).split(sep).join('/');

  if (!file.endsWith('.md')) {
    problems.push(`${where}: only .md files belong under docs/site/`);
    continue;
  }

  const fields = frontmatter(readFileSync(file, 'utf8'));
  if (!fields) {
    problems.push(`${where}: no frontmatter block at the top of the file`);
    continue;
  }

  for (const key of ['title', 'description']) {
    if (!fields.get(key)) {
      problems.push(`${where}: frontmatter needs a non-empty ${key}`);
    }
  }
}

if (versions.length === 0 || files.length === 0) {
  problems.push('docs/site/ is empty, which would ship a site with no documentation');
}

if (problems.length > 0) {
  console.error(problems.map((p) => `  ${p}`).join('\n'));
  console.error(`\n${problems.length} problem(s) across ${files.length} file(s)`);
  process.exit(1);
}

console.log(`${files.length} pages validated`);
