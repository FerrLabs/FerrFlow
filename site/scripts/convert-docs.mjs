import { readdirSync, readFileSync, mkdirSync, writeFileSync, statSync } from 'node:fs';
import { join, relative, dirname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { marked } from 'marked';

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

const ASIDE_TYPE_CLASS = {
  note: 'note',
  tip: 'tip',
  info: 'info',
  caution: 'warning',
  danger: 'warning',
  warning: 'warning',
};

function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) out.push(...walk(full));
    else if (entry.endsWith('.mdx')) out.push(full);
  }
  return out;
}

function splitFrontmatter(raw) {
  const m = raw.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?/);
  if (!m) return { frontmatter: '', body: raw };
  return { frontmatter: m[0], body: raw.slice(m[0].length) };
}

function dropImports(body) {
  return body.replace(/^import\s+.*\sfrom\s+['"].*['"];?\s*$/gm, '');
}

function md(inner) {
  return marked.parse(dedent(inner).trim());
}

function dedent(text) {
  const lines = text.replace(/^\r?\n/, '').split('\n');
  const indents = lines.filter((l) => l.trim().length > 0).map((l) => l.match(/^\s*/)[0].length);
  const min = indents.length ? Math.min(...indents) : 0;
  return lines.map((l) => l.slice(min)).join('\n');
}

function findMatching(body, tag, startIdx) {
  const openRe = new RegExp(`<${tag}(\\s[^>]*?)?>`, 'g');
  const closeRe = new RegExp(`</${tag}>`, 'g');
  let depth = 1;
  let pos = startIdx;
  while (depth > 0) {
    openRe.lastIndex = pos;
    closeRe.lastIndex = pos;
    const open = openRe.exec(body);
    const close = closeRe.exec(body);
    if (!close) throw new Error(`Unbalanced <${tag}>`);
    if (open && open.index < close.index) {
      depth++;
      pos = open.index + open[0].length;
    } else {
      depth--;
      pos = close.index + close[0].length;
      if (depth === 0) return { innerStart: startIdx, innerEnd: close.index, end: pos };
    }
  }
}

function getAttr(attrs, name) {
  if (!attrs) return undefined;
  const m = attrs.match(new RegExp(`${name}=("([^"]*)"|'([^']*)'|\\{([^}]*)\\})`));
  if (!m) return undefined;
  return (m[2] ?? m[3] ?? m[4] ?? '').trim();
}

function aside(typeClass, title, innerHtml) {
  const titleEl = title ? `<p class="ferr-aside__title">${title}</p>` : '';
  return `<aside class="ferr-aside ferr-aside--${typeClass}">${titleEl}<div class="ferr-aside__body">${innerHtml}</div></aside>`;
}

function card(title, href, innerHtml) {
  const open = href ? `<a class="ferr-card" href="${href}">` : '<div class="ferr-card">';
  const close = href ? '</a>' : '</div>';
  return `${open}<p class="ferr-card__title">${title}</p><div class="ferr-card__body">${innerHtml}</div>${close}`;
}

function convertCards(segment) {
  const re = /<Card(\s[^>]*?)?>/g;
  let out = '';
  let cursor = 0;
  let m;
  while ((m = re.exec(segment))) {
    out += segment.slice(cursor, m.index);
    const attrs = m[1] || '';
    const { innerStart, innerEnd, end } = findMatching(segment, 'Card', m.index + m[0].length);
    const title = getAttr(attrs, 'title') ?? '';
    const href = getAttr(attrs, 'href');
    const inner = segment.slice(innerStart, innerEnd);
    out += card(title, href, md(inner));
    cursor = end;
    re.lastIndex = end;
  }
  out += segment.slice(cursor);
  return out;
}

function convertTabItems(segment) {
  const re = /<TabItem(\s[^>]*?)?>/g;
  let out = '';
  let cursor = 0;
  let m;
  while ((m = re.exec(segment))) {
    out += segment.slice(cursor, m.index);
    const attrs = m[1] || '';
    const { innerStart, innerEnd, end } = findMatching(segment, 'TabItem', m.index + m[0].length);
    const label = getAttr(attrs, 'label') ?? '';
    const inner = segment.slice(innerStart, innerEnd);
    out += `<div class="ferr-tab" data-label="${label}"><p class="ferr-tab__label">${label}</p><div class="ferr-tab__body">${md(inner)}</div></div>`;
    cursor = end;
    re.lastIndex = end;
  }
  out += segment.slice(cursor);
  return out;
}

const BLOCK_HANDLERS = [
  {
    tag: 'CardGroup',
    re: /<CardGroup(\s[^>]*?)?>/,
    render(attrs, inner) {
      const cols = getAttr(attrs, 'cols') ?? '2';
      return `<div class="ferr-card-group" data-cols="${cols}">${convertCards(inner)}</div>`;
    },
  },
  {
    tag: 'Tabs',
    re: /<Tabs(\s[^>]*?)?>/,
    render(attrs, inner) {
      return `<div class="ferr-tabs">${convertTabItems(inner)}</div>`;
    },
  },
  {
    tag: 'Steps',
    re: /<Steps(\s[^>]*?)?>/,
    render(attrs, inner) {
      return md(inner);
    },
  },
  {
    tag: 'Aside',
    re: /<Aside(\s[^>]*?)?>/,
    render(attrs, inner) {
      const type = (getAttr(attrs, 'type') ?? 'note').toLowerCase();
      const cls = ASIDE_TYPE_CLASS[type] ?? 'note';
      const title = getAttr(attrs, 'title');
      return aside(cls, title, md(inner));
    },
  },
  {
    tag: 'Note',
    re: /<Note(\s[^>]*?)?>/,
    render(attrs, inner) {
      return aside('note', getAttr(attrs, 'title'), md(inner));
    },
  },
  {
    tag: 'Info',
    re: /<Info(\s[^>]*?)?>/,
    render(attrs, inner) {
      return aside('info', getAttr(attrs, 'title'), md(inner));
    },
  },
  {
    tag: 'Tip',
    re: /<Tip(\s[^>]*?)?>/,
    render(attrs, inner) {
      return aside('tip', getAttr(attrs, 'title'), md(inner));
    },
  },
  {
    tag: 'Warning',
    re: /<Warning(\s[^>]*?)?>/,
    render(attrs, inner) {
      return aside('warning', getAttr(attrs, 'title'), md(inner));
    },
  },
];

function convertComponents(body) {
  for (;;) {
    let best = null;
    for (const handler of BLOCK_HANDLERS) {
      const m = handler.re.exec(body);
      if (m && (best === null || m.index < best.match.index)) {
        best = { handler, match: m };
      }
    }
    if (!best) break;

    const { handler, match } = best;
    const openEnd = match.index + match[0].length;
    const { innerStart, innerEnd, end } = findMatching(body, handler.tag, openEnd);
    const attrs = match[1] || '';
    const inner = body.slice(innerStart, innerEnd);
    const replacement = handler.render(attrs, inner);
    body = body.slice(0, match.index) + replacement + body.slice(end);
  }
  return body;
}

function convertAsideDirectives(body) {
  const re = /^:::(\w+)(?:\[([^\]]*)\])?\s*\r?\n([\s\S]*?)\r?\n:::\s*$/gm;
  return body.replace(re, (_full, type, title, inner) => {
    const cls = ASIDE_TYPE_CLASS[type.toLowerCase()] ?? 'note';
    return `${aside(cls, title || undefined, md(inner))}\n`;
  });
}

function convert(raw) {
  const { frontmatter, body } = splitFrontmatter(raw);
  let work = dropImports(body);
  work = convertAsideDirectives(work);
  work = convertComponents(work);
  work = work.replace(/\n{3,}/g, '\n\n');
  return frontmatter + work.replace(/^\n+/, '');
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

generated.sort();
for (const g of generated) console.log(g);
console.log(
  `\nConverted ${total} files (${SOURCES.map((s) => relative(OUT, s.outDir)).join(' + ')}).`,
);
