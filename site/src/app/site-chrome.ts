import type { SiteChromeConfig } from '@ferrlabs/ui-ng';

const LOGO_SVG = `<svg viewBox="0 0 32 32" fill="none" aria-hidden="true">
  <line x1="6" y1="16" x2="14" y2="16" stroke="currentColor" stroke-width="1.5" />
  <line x1="18" y1="16" x2="24" y2="10" stroke="currentColor" stroke-width="1.5" />
  <line x1="18" y1="16" x2="24" y2="22" stroke="currentColor" stroke-width="1.5" />
  <circle cx="4" cy="16" r="3" fill="currentColor" />
  <circle cx="16" cy="16" r="3" fill="currentColor" />
  <circle cx="26" cy="10" r="3" fill="currentColor" opacity="0.6" />
  <circle cx="26" cy="22" r="3" fill="currentColor" opacity="0.6" />
</svg>`;

export function ferrflowChrome(): SiteChromeConfig {
  return {
    origin: 'https://ferrflow.com',
    logoSvg: LOGO_SVG,
    wordmark: 'ferrflow',
    navLinks: [
      { label: $localize`:@@nav.docs:Docs`, href: '/docs/introduction/' },
      { label: $localize`:@@nav.performance:Performance`, href: '/performance/' },
      { label: $localize`:@@nav.why:Why FerrFlow`, href: '/why/' },
      { label: $localize`:@@nav.changelog:Changelog`, href: '/changelog/' },
      {
        label: $localize`:@@nav.github:GitHub`,
        href: 'https://github.com/FerrLabs/FerrFlow',
        external: true,
      },
    ],
    cta: { label: $localize`:@@nav.cta:Get FerrFlow`, href: '/docs/introduction/' },
    footer: {
      tagline: $localize`:@@footer.tagline:Conventional commits in, semantic versions out. One Rust binary, every monorepo.`,
      backLabel: $localize`:@@footer.back:← Back to ferrlabs.com`,
      backHref: 'https://ferrlabs.com',
      columns: [
        {
          title: $localize`:@@footer.resources:Resources`,
          links: [
            {
              label: $localize`:@@footer.documentation:Documentation`,
              href: '/docs/introduction/',
            },
            { label: $localize`:@@footer.performance:Performance`, href: '/performance/' },
            { label: $localize`:@@footer.changelog:Changelog`, href: '/changelog/' },
            { label: $localize`:@@footer.contact:Contact`, href: '/contact/' },
            {
              label: $localize`:@@footer.releases:Releases`,
              href: 'https://github.com/FerrLabs/FerrFlow/releases',
              external: true,
            },
            {
              label: $localize`:@@footer.discussions:Discussions`,
              href: 'https://github.com/FerrLabs/FerrFlow/discussions',
              external: true,
            },
          ],
        },
        {
          title: $localize`:@@footer.product:Product`,
          links: [
            { label: 'GitHub', href: 'https://github.com/FerrLabs/FerrFlow', external: true },
            { label: 'FerrLabs', href: 'https://ferrlabs.com', external: true },
          ],
        },
        {
          title: $localize`:@@footer.legal:Legal`,
          links: [
            { label: $localize`:@@footer.mentions:Legal notice`, href: '/legal/' },
            { label: $localize`:@@footer.privacy:Privacy`, href: '/privacy/' },
            { label: $localize`:@@footer.cgu:Terms of use`, href: '/terms/' },
            { label: $localize`:@@footer.cookies:Cookies`, href: '/cookies/' },
            { label: $localize`:@@footer.security:Security`, href: '/security/' },
          ],
        },
      ],
      bottomLeft: $localize`:@@footer.rights:© 2026 FerrLabs. FerrFlow is a FerrLabs product.`,
      bottomRight: 'SIREN 104 243 951',
    },
    labels: {
      menu: $localize`:@@nav.menu:Menu`,
      close: $localize`:@@nav.close:Close`,
      openMenu: $localize`:@@nav.openMenu:Open menu`,
    },
  };
}
