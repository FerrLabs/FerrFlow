import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const FRONTMATTER = /^---\r?\n[\s\S]*?\r?\n---/;
const SLUG_LINE = /^slug:.*\r?\n/m;

const PRE_BLOCK = /<pre\b[\s\S]*?<\/pre>/g;
const BLANK_LINE = /\n[ \t]*(?=\n)/g;

const INTERNAL_LINK = /(\]\(|href=")(\/(?!\/)[^)"\s?#]*)([?#][^)"\s]*)?/g;

export function slashInternalLinks(dir) {
  return readdirSync(dir, { withFileTypes: true }).reduce((total, entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      return total + slashInternalLinks(full);
    }
    if (!entry.name.endsWith('.md')) {
      return total;
    }
    let count = 0;
    const markdown = readFileSync(full, 'utf8').replace(
      INTERNAL_LINK,
      (link, opening, path, suffix = '') => {
        const lastSegment = path.slice(path.lastIndexOf('/') + 1);
        if (path.endsWith('/') || lastSegment.includes('.')) {
          return link;
        }
        count += 1;
        return `${opening}${path}/${suffix}`;
      },
    );
    if (count > 0) {
      writeFileSync(full, markdown);
    }
    return total + count;
  }, 0);
}

export function keepPreBlocksWhole(dir) {
  return readdirSync(dir, { withFileTypes: true }).reduce((total, entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      return total + keepPreBlocksWhole(full);
    }
    if (!entry.name.endsWith('.md')) {
      return total;
    }
    let count = 0;
    const markdown = readFileSync(full, 'utf8').replace(PRE_BLOCK, (block) => {
      const kept = block.replace(BLANK_LINE, '\n<span></span>');
      if (kept !== block) {
        count += 1;
      }
      return kept;
    });
    if (count > 0) {
      writeFileSync(full, markdown);
    }
    return total + count;
  }, 0);
}

export function dropSlugOverride(dir) {
  return readdirSync(dir, { withFileTypes: true }).reduce((total, entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      return total + dropSlugOverride(full);
    }
    if (!entry.name.endsWith('.md')) {
      return total;
    }
    const markdown = readFileSync(full, 'utf8');
    const head = FRONTMATTER.exec(markdown)?.[0];
    if (head === undefined || !SLUG_LINE.test(head)) {
      return total;
    }
    writeFileSync(full, markdown.replace(head, head.replace(SLUG_LINE, '')));
    return total + 1;
  }, 0);
}

export function countPages(dir) {
  return readdirSync(dir, { withFileTypes: true }).reduce((total, entry) => {
    if (entry.isDirectory()) {
      return total + countPages(join(dir, entry.name));
    }
    return entry.name.endsWith('.md') ? total + 1 : total;
  }, 0);
}
