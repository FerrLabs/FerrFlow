import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Resvg } from '@resvg/resvg-js';
import satori from 'satori';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const WIDTH = 1200;
const HEIGHT = 630;
const ACCENT = '#e8733a';
const INK = '#1e293b';
const INK_2 = '#475569';
const INK_3 = '#64748b';
const PAPER = '#faf8f4';
const RULE = 'rgba(30, 41, 59, 0.14)';

const font = (family, name, weight) => ({
  name: family,
  weight,
  style: 'normal',
  data: readFileSync(
    join(root, `node_modules/@fontsource/${name}/files/${name}-latin-${weight}-normal.woff`),
  ),
});

const FONTS = [
  font('Fraunces', 'fraunces', 400),
  font('Fraunces', 'fraunces', 700),
  font('DM Mono', 'dm-mono', 400),
];

const LOGO = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" fill="none">
  <line x1="6" y1="16" x2="14" y2="16" stroke="${ACCENT}" stroke-width="1.5" />
  <line x1="18" y1="16" x2="24" y2="10" stroke="${ACCENT}" stroke-width="1.5" />
  <line x1="18" y1="16" x2="24" y2="22" stroke="${ACCENT}" stroke-width="1.5" />
  <circle cx="4" cy="16" r="3" fill="${ACCENT}" />
  <circle cx="16" cy="16" r="3" fill="${ACCENT}" />
  <circle cx="26" cy="10" r="3" fill="${ACCENT}" opacity="0.6" />
  <circle cx="26" cy="22" r="3" fill="${ACCENT}" opacity="0.6" />
</svg>`;
const LOGO_URI = `data:image/svg+xml;base64,${Buffer.from(LOGO).toString('base64')}`;

const el = (type, style, children) => ({ type, props: { style, children } });

function titleSize(title) {
  if (title.length > 80) return 48;
  if (title.length > 40) return 58;
  return 72;
}

function brand() {
  return el('div', { display: 'flex', alignItems: 'center', gap: 20 }, [
    { type: 'img', props: { src: LOGO_URI, width: 56, height: 56 } },
    el('div', { fontFamily: 'DM Mono', fontSize: 38, color: INK }, 'ferrflow'),
  ]);
}

function body({ title, description }) {
  return el('div', { display: 'flex', flexDirection: 'column', gap: 28 }, [
    el(
      'div',
      {
        fontFamily: 'Fraunces',
        fontWeight: 700,
        fontSize: titleSize(title),
        lineHeight: 1.08,
        color: INK,
        lineClamp: 3,
      },
      title,
    ),
    description
      ? el(
          'div',
          { fontFamily: 'Fraunces', fontSize: 28, lineHeight: 1.35, color: INK_2, lineClamp: 3 },
          description,
        )
      : null,
  ]);
}

function footer(path) {
  return el(
    'div',
    {
      display: 'flex',
      borderTop: `2px solid ${RULE}`,
      paddingTop: 28,
      fontFamily: 'DM Mono',
      fontSize: 24,
      color: INK_3,
      lineClamp: 1,
    },
    `ferrflow.com${path}`,
  );
}

function card(job) {
  return el(
    'div',
    {
      display: 'flex',
      flexDirection: 'column',
      justifyContent: 'space-between',
      width: WIDTH,
      height: HEIGHT,
      boxSizing: 'border-box',
      padding: '56px 72px',
      background: PAPER,
      borderLeft: `16px solid ${ACCENT}`,
    },
    [brand(), body(job), footer(job.path)],
  );
}

export async function renderCard(job) {
  const svg = await satori(card(job), { width: WIDTH, height: HEIGHT, fonts: FONTS });
  return new Resvg(svg).render().asPng();
}
