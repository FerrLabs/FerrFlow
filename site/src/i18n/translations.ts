export const languages = {
  en: 'English',
  fr: 'Français',
} as const;

export type Lang = keyof typeof languages;

export const defaultLang: Lang = 'en';

export function getLangFromUrl(url: URL): Lang {
  const [, lang] = url.pathname.split('/');
  if (lang in languages) return lang as Lang;
  return defaultLang;
}

export function getLocalizedPath(path: string, lang: Lang): string {
  if (lang === defaultLang) return path;
  return `/${lang}${path}`;
}

const ui = {
  en: {
    'meta.title': 'FerrFlow — Universal semantic versioning CLI',
    'meta.description':
      'FerrFlow is an open-source CLI that bumps versions, generates changelogs, and ships releases — across 14+ file formats, monorepos, and CI. One Rust binary, any language, git-driven.',

    'nav.docs': 'Docs',
    'nav.cta': 'Get FerrFlow',
    'lang.unavailable': 'Not yet translated',

    'hero.badge': 'Open source · v4',
    'hero.h1': 'Release.',
    'hero.h2': 'Automatically.',
    'hero.sub':
      'Conventional commits in, semantic versions out. FerrFlow reads your git history, bumps versions across your monorepo, writes changelogs, tags, and ships GitHub releases — straight from CI.',
    'hero.cta1': 'cargo install ferrflow',
    'hero.cta2': 'Read the docs',
    'hero.stat.latest': 'Latest',
    'hero.stat.license': 'License',
    'hero.stat.lang': 'Lang',
    'hero.stat.size': 'Size',

    'terminal.reading': 'reading 184 commits since last release...',
    'terminal.detected': 'detected: 12 features, 9 fixes, 1 breaking',
    'terminal.scanning': 'scanning 4 packages...',
    'terminal.generating': 'generating changelogs, tagging, pushing...',
    'terminal.published': '4 GitHub releases published · 0.42s',

    'formats.label': 'Reads natively',

    'features.eyebrow': '№ 02 — Features',
    'features.title': 'Everything you need to release.',
    'features.sub': 'Single binary. Zero plugins. Drop into any CI and go.',
    'features.01.title': 'Semantic versioning',
    'features.01.body':
      'Reads conventional commits and determines the right bump. Supports semver, calver, sequential, and zerover.',
    'features.02.title': 'Monorepo native',
    'features.02.body':
      'Version multiple packages independently with shared paths, dependency chains, and per-package changelogs.',
    'features.03.title': 'Multi-forge',
    'features.03.body':
      'GitHub, GitLab, self-hosted forges. Tags, releases, PR preview comments — natively supported.',
    'features.04.title': '14+ version files',
    'features.04.body':
      'Cargo.toml, package.json, pyproject.toml, Chart.yaml, mix.exs, gemspec, pubspec.yaml — bumped automatically.',
    'features.05.title': 'Pre-release channels',
    'features.05.body':
      'Branch-based or flag-based: alpha, beta, rc, dev. Floating tags never moved by pre-releases.',
    'features.06.title': 'Open source',
    'features.06.body':
      'MPL-2.0. Self-host, fork, audit. The Rust binary handles GitHub OIDC directly — minimal runners work out of the box.',

    'install.eyebrow': '№ 03 — Install',
    'install.title': 'Install in seconds.',
    'install.sub':
      'Works on Linux, macOS, and Windows. Available as a GitHub Action and a GitLab CI template.',

    'cta.eyebrow': '№ 04 — Get started',
    'cta.title1': 'Ready to ship',
    'cta.title2': 'without the ceremony?',
    'cta.subtitle': 'Open source. Runs in your own CI. Your code never leaves your infrastructure.',
    'cta.button': 'View on GitHub',
    'cta.installShort': 'Install',

    'footer.rights': '© 2026 FerrLabs. FerrFlow is a FerrLabs product.',
    'footer.tagline':
      'Conventional commits in, semantic versions out. One Rust binary, every monorepo.',
    'footer.back': '← Back to ferrlabs.com',
    'footer.documentation': 'Documentation',
    'footer.releases': 'Releases',
    'footer.discussions': 'Discussions',
    'footer.mentions': 'Legal notice',
    'footer.terms': 'Terms',
    'footer.privacy': 'Privacy',
    'footer.cookies': 'Cookies',
  },
  fr: {
    'meta.title': 'FerrFlow — CLI universel de versioning sémantique',
    'meta.description':
      'FerrFlow est un CLI open source qui bump les versions, génère les changelogs et publie les releases — sur 14+ formats de fichiers, monorepos et CI. Un seul binaire Rust, tous langages, piloté par git.',

    'nav.docs': 'Docs',
    'nav.cta': 'Obtenir FerrFlow',
    'lang.unavailable': 'Pas encore traduit',

    'hero.badge': 'Open source · v4',
    'hero.h1': 'Publiez.',
    'hero.h2': 'Automatiquement.',
    'hero.sub':
      'Des conventional commits en entrée, des versions sémantiques en sortie. FerrFlow lit votre historique git, bump les versions de votre monorepo, écrit les changelogs, tag et publie les releases — directement depuis la CI.',
    'hero.cta1': 'cargo install ferrflow',
    'hero.cta2': 'Lire la doc',
    'hero.stat.latest': 'Dernière',
    'hero.stat.license': 'Licence',
    'hero.stat.lang': 'Langage',
    'hero.stat.size': 'Taille',

    'terminal.reading': 'lecture de 184 commits depuis la dernière release...',
    'terminal.detected': 'détecté : 12 features, 9 fixes, 1 breaking',
    'terminal.scanning': 'analyse de 4 packages...',
    'terminal.generating': 'génération des changelogs, tagging, push...',
    'terminal.published': '4 releases GitHub publiées · 0.42s',

    'formats.label': 'Lit nativement',

    'features.eyebrow': '№ 02 — Fonctionnalités',
    'features.title': 'Tout pour livrer vos releases.',
    'features.sub': "Un seul binaire. Zéro plugin. Branchez sur la CI et c'est parti.",
    'features.01.title': 'Versioning sémantique',
    'features.01.body':
      'Lit les commits conventionnels et détermine le bon bump. Stratégies semver, calver, sequential et zerover.',
    'features.02.title': 'Monorepo natif',
    'features.02.body':
      'Versionnez plusieurs packages indépendamment, avec chemins partagés, dépendances et changelogs par package.',
    'features.03.title': 'Multi-forge',
    'features.03.body':
      'GitHub, GitLab, forges self-hosted. Tags, releases, commentaires PR — nativement supportés.',
    'features.04.title': '14+ formats de version',
    'features.04.body':
      'Cargo.toml, package.json, pyproject.toml, Chart.yaml, mix.exs, gemspec, pubspec.yaml — bumpés automatiquement.',
    'features.05.title': 'Canaux pré-release',
    'features.05.body':
      'Par branche ou par flag : alpha, beta, rc, dev. Les tags flottants ne sont jamais bougés par les pré-releases.',
    'features.06.title': 'Open source',
    'features.06.body':
      'MPL-2.0. Self-host, forkez, auditez. Le binaire Rust gère OIDC GitHub — les runners minimaux fonctionnent immédiatement.',

    'install.eyebrow': '№ 03 — Installer',
    'install.title': 'Installé en quelques secondes.',
    'install.sub': 'Linux, macOS et Windows. Disponible en GitHub Action et en template GitLab CI.',

    'cta.eyebrow': '№ 04 — Démarrer',
    'cta.title1': 'Prêt à publier',
    'cta.title2': 'sans cérémonie ?',
    'cta.subtitle':
      'Open source. Tourne dans votre propre CI. Aucune donnée ne quitte votre infrastructure.',
    'cta.button': 'Voir sur GitHub',
    'cta.installShort': 'Installer',

    'footer.rights': '© 2026 FerrLabs. FerrFlow est un produit FerrLabs.',
    'footer.tagline':
      'Des conventional commits en entrée, des versions sémantiques en sortie. Un binaire Rust, tous les monorepos.',
    'footer.back': '← Retour à ferrlabs.com',
    'footer.documentation': 'Documentation',
    'footer.releases': 'Releases',
    'footer.discussions': 'Discussions',
    'footer.mentions': 'Mentions légales',
    'footer.terms': 'Conditions',
    'footer.privacy': 'Confidentialité',
    'footer.cookies': 'Cookies',
  },
} as const;

export function t(lang: Lang, key: keyof typeof ui.en | string): string {
  const dict = ui[lang] as Record<string, string>;
  const fallback = ui.en as Record<string, string>;
  return dict[key as string] ?? fallback[key as string] ?? '';
}
