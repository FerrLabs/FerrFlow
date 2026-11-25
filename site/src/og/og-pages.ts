export const SITE = 'https://ferrflow.com';

export interface PageMeta {
  title: string;
  description: string;
  canonicalPath: string | undefined;
}

export interface CardJob {
  imagePath: string;
  title: string;
  description: string;
  path: string;
}

export interface Page {
  path: string;
  meta: PageMeta | undefined;
}

export interface ImagePlan {
  jobs: CardJob[];
  imageByPage: Map<string, string>;
}

const NAMED_ENTITIES: Record<string, string> = {
  amp: '&',
  lt: '<',
  gt: '>',
  quot: '"',
  apos: "'",
};

const TITLE_DECORATIONS =
  /^FerrFlow: |(?: \| (?:FerrFlow Changelog|Changelog FerrFlow)|: FerrFlow(?: Docs)?)$/g;
const NOT_FOUND = /^\/(?:(?:fr|en)\/)?404\/$/;
const DEFAULT_LOCALE_PREFIX = '/en/';
const OG_IMAGE_TAGS = [
  /(<meta property="og:image" content=")[^"]*(")/,
  /(<meta name="twitter:image" content=")[^"]*(")/,
];

function decodeEntities(text: string): string {
  return text.replace(/&(?:#(\d+)|#x([0-9a-f]+)|(\w+));/gi, (match, dec, hex, name) => {
    if (dec) return String.fromCodePoint(Number(dec));
    if (hex) return String.fromCodePoint(parseInt(hex, 16));
    return NAMED_ENTITIES[name.toLowerCase()] ?? match;
  });
}

function metaContent(html: string, attribute: string): string | undefined {
  const raw = new RegExp(`<meta ${attribute} content="([^"]*)"`).exec(html)?.[1];
  return raw === undefined ? undefined : decodeEntities(raw);
}

function canonicalPathOf(html: string): string | undefined {
  const href = /<link rel="canonical" href="([^"]+)"/.exec(html)?.[1];
  if (href === undefined || !href.startsWith(`${SITE}/`)) return undefined;
  return href.slice(SITE.length);
}

export function readPageMeta(html: string): PageMeta | undefined {
  const title = metaContent(html, 'property="og:title"');
  if (!title) return undefined;
  return {
    title,
    description: metaContent(html, 'property="og:description"') ?? '',
    canonicalPath: canonicalPathOf(html),
  };
}

export function cardTitle(title: string): string {
  return title.replace(TITLE_DECORATIONS, '') || title;
}

export function imagePathFor(pagePath: string): string {
  const slug = pagePath.replace(/^\/|\/$/g, '');
  return `/og/${slug || 'index'}.png`;
}

export function imageUrl(imagePath: string): string {
  return `${SITE}${imagePath}`;
}

export function withImage(html: string, imagePath: string): string {
  const url = imageUrl(imagePath);
  return OG_IMAGE_TAGS.reduce((out, tag) => out.replace(tag, `$1${url}$2`), html);
}

function ownerOf(path: string, withMeta: Map<string, PageMeta>): string {
  const unprefixed = path.startsWith(DEFAULT_LOCALE_PREFIX)
    ? path.slice(DEFAULT_LOCALE_PREFIX.length - 1)
    : undefined;
  const start = unprefixed !== undefined && withMeta.has(unprefixed) ? unprefixed : path;
  const canonical = withMeta.get(start)!.canonicalPath;
  return canonical !== undefined && withMeta.has(canonical) ? canonical : start;
}

export function planImages(pages: Page[]): ImagePlan {
  const withMeta = new Map(
    pages.flatMap((page) =>
      page.meta && !NOT_FOUND.test(page.path) ? [[page.path, page.meta]] : [],
    ),
  );
  const imageByPage = new Map<string, string>();
  const jobs = new Map<string, CardJob>();
  for (const path of withMeta.keys()) {
    const owner = ownerOf(path, withMeta);
    const imagePath = imagePathFor(owner);
    imageByPage.set(path, imagePath);
    if (!jobs.has(imagePath)) {
      const ownerMeta = withMeta.get(owner)!;
      jobs.set(imagePath, {
        imagePath,
        title: cardTitle(ownerMeta.title),
        description: ownerMeta.description,
        path: owner,
      });
    }
  }
  return { jobs: [...jobs.values()], imageByPage };
}
