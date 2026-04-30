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
    // Meta
    'meta.title': 'FerrFlow — Universal Semantic Versioning',
    'meta.description': 'Automated semantic versioning, changelogs, and releases for monorepos and classic repos. One binary, any language, git-driven.',

    // Nav
    'nav.docs': 'Docs',
    // 'nav.playground': 'Playground', // disabled — playground is temporarily unavailable
    'nav.signin': 'Sign in',
    'nav.signup': 'Sign up',
    'nav.products.cli.desc': 'One binary for versioning and releases',
    'lang.unavailable': 'Not yet translated',

    // Hero
    'hero.headline1': 'Ship faster.',
    'hero.headline2': 'Stay versioned.',
    'hero.subheading': 'FerrFlow automates semantic versioning, changelogs, and releases from your conventional commits. One binary, any language, any CI.',
    'hero.cta.download': 'Download latest release',
    'hero.cta.docs': 'Read the docs',

    // Products
    'products.title': 'Versioning, solved',
    'products.subtitle': 'FerrFlow reads your commits, bumps versions, writes changelogs, tags, and releases — without ceremony.',
    'products.versioning.name': 'CLI',
    'products.versioning.desc': 'Monorepo-native releases driven by conventional commits. Supports semver, calver, sequential, and zerover strategies.',
    'products.versioning.cta': 'Get started',

    // Terminal
    'terminal.scanning': 'Scanning packages/api ...',

    // Features
    'features.title': 'Why FerrFlow?',
    'features.subtitle': 'A release tool built for the way you actually ship.',
    'features.monorepo.title': 'CLI-first',
    'features.monorepo.desc': 'Everything happens from your terminal or your CI. Zero UI to click, zero config server to babysit.',
    'features.agnostic.title': 'Multi-forge',
    'features.agnostic.desc': 'GitHub, GitLab, self-hosted — FerrFlow adapts to your forge. One tool, any platform.',
    'features.commits.title': 'Conventional commits',
    'features.commits.desc': 'Reads your commit history to determine version bumps automatically. No manual changelog maintenance.',
    'features.changelog.title': 'Zero infra',
    'features.changelog.desc': 'A single binary with no daemon, no server, no database. Runs wherever your CI runs.',
    'features.ci.title': 'Open source',
    'features.ci.desc': 'Fully open source. Self-host, fork, audit — your choice.',
    'features.private.title': 'Stays private',
    'features.private.desc': 'Your code never leaves your infrastructure. FerrFlow runs entirely inside your own pipeline.',

    // How it works
    'how.title': 'How it works',
    'how.step1.title': 'Install the CLI',
    'how.step1.desc': 'One binary, all features. Works on Linux, macOS, and Windows.',
    'how.step2.title': 'Connect your project',
    'how.step2.desc': 'FerrFlow detects your repo, forge, and config automatically.',
    'how.step3.title': 'Ship with confidence',
    'how.step3.desc': 'Bump versions, generate changelogs, tag and release — all from a single command.',

    // CTA
    'cta.title1': 'Ready to ship',
    'cta.title2': 'without the ceremony?',
    'cta.subtitle': 'Open source. Runs in your own CI. No data leaves your infrastructure.',
    'cta.button': 'View on GitHub',

    // Pre-footer CTA (shared ui-astro component) — DESIGN.md §4.1 (FerrFlow row).
    'prefooter.title': 'Ready to ship.',
    'prefooter.subtitle': 'Without the ceremony.',

    // Footer
    'footer.rights': '© 2026 FerrLabs. All rights reserved.',
    'footer.documentation': 'Documentation',
    'footer.releases': 'Releases',
    'footer.product': 'Product',
    'footer.resources': 'Resources',
    'footer.ferrlabs': 'FerrLabs',
    'footer.legal': 'Legal',
    'footer.reportIssue': 'Report an issue',
    'footer.discussions': 'Discussions',
    'footer.terms': 'Terms of service',
    'footer.privacy': 'Privacy policy',


    // Landing page copy — rendered by src/components/LandingPage.astro on both / and /fr/.
    'cliPage.meta.title': 'FerrFlow CLI',
    'cliPage.meta.description': 'A single Rust binary for automated semantic versioning, changelogs, and releases. Monorepo-native, conventional-commit driven, runs in any CI.',
    'cliPage.hero.badge': 'Open source',
    'cliPage.hero.headline1': 'Release.',
    'cliPage.hero.headline2': 'Automatically.',
    'cliPage.hero.subheading': 'Conventional commits in, semantic versions out. FerrFlow reads your git history, bumps versions across your monorepo, writes changelogs, tags, and ships GitHub releases — straight from CI. Just versioning, done right.',
    'cliPage.hero.download': 'Download',
    'cliPage.hero.docs': 'Documentation',
    'cliPage.features.title': 'Everything you need to release',
    'cliPage.features.subtitle': 'Single binary, zero plugins. Drop into any CI and go.',
    'cliPage.features.versioning.title': 'Semantic versioning',
    'cliPage.features.versioning.desc': 'Reads conventional commits and determines the right bump. Supports semver, calver, sequential, and zerover strategies.',
    'cliPage.features.monorepo.title': 'Monorepo native',
    'cliPage.features.monorepo.desc': 'Version multiple packages independently with shared paths, dependency chains, and per-package changelogs.',
    'cliPage.features.changelog.title': 'Changelog generation',
    'cliPage.features.changelog.desc': 'Structured, readable changelogs generated from your commits. Grouped by type, scoped per package.',
    'cliPage.features.forge.title': 'Multi-forge',
    'cliPage.features.forge.desc': 'GitHub, GitLab, self-hosted forges. Tags, releases, and PR preview comments — natively supported.',
    'cliPage.features.formats.title': '14+ version files',
    'cliPage.features.formats.desc': 'Cargo.toml, package.json, pyproject.toml, Chart.yaml, mix.exs, gemspec, pubspec.yaml, and more — bumped automatically.',
    'cliPage.terminal.scanning': 'Scanning 4 packages, 12 commits since last release...',
    'cliPage.terminal.bump': 'Bump ',
    'cliPage.terminal.tag': 'Tag  ',
    'cliPage.terminal.generating': 'Generating changelogs, tagging, pushing...',
    'cliPage.terminal.released': '✓ 2 GitHub releases published',
    'cliPage.install.title': 'Install in seconds',
    'cliPage.install.works': 'Works on Linux, macOS, and Windows. Also available as a ',
    'cliPage.install.ghAction': 'GitHub Action',
    'cliPage.install.and': ' and ',
    'cliPage.install.glCi': 'GitLab CI template',
    'cliPage.install.dot': '.',


  },
  fr: {
    // Meta
    'meta.title': 'FerrFlow — Plateforme D\u00e9veloppeur',
    'meta.description': 'Versioning s\u00e9mantique automatis\u00e9, changelogs et releases pour monorepos et repos classiques. Un seul binaire, tous langages, pilot\u00e9 par git.',

    // Nav
    'nav.docs': 'Docs',
    // 'nav.playground': 'Playground', // disabled — playground is temporarily unavailable
    'nav.signin': 'Connexion',
    'nav.signup': 'Inscription',
    'nav.products.cli.desc': 'Un binaire pour le versioning et les releases',
    'lang.unavailable': 'Pas encore traduit',

    // Hero
    'hero.headline1': 'Livrez vite.',
    'hero.headline2': 'Restez versionn\u00e9.',
    'hero.subheading': "FerrFlow automatise le versioning s\u00e9mantique, les changelogs et les releases depuis vos commits conventionnels. Un binaire, tout langage, toute CI.",
    'hero.cta.download': 'T\u00e9l\u00e9charger la derni\u00e8re version',
    'hero.cta.docs': 'Lire la documentation',

    // Products
    'products.title': 'Le versioning, r\u00e9solu',
    'products.subtitle': 'FerrFlow lit vos commits, bump les versions, \u00e9crit les changelogs, tag et release \u2014 sans c\u00e9r\u00e9monie.',
    'products.versioning.name': 'CLI',
    'products.versioning.desc': 'Releases monorepo-native pilot\u00e9es par conventional commits. Supporte semver, calver, s\u00e9quentielle et zerover.',
    'products.versioning.cta': 'Commencer',

    // Terminal
    'terminal.scanning': 'Analyse de packages/api ...',

    // Features
    'features.title': 'Pourquoi FerrFlow ?',
    'features.subtitle': 'Un outil de release con\u00e7u pour votre fa\u00e7on de livrer.',
    'features.monorepo.title': 'CLI-first',
    'features.monorepo.desc': 'Tout se passe dans le terminal ou votre CI. Pas d\'UI \u00e0 cliquer, pas de serveur de config \u00e0 g\u00e9rer.',
    'features.agnostic.title': 'Multi-forge',
    'features.agnostic.desc': 'GitHub, GitLab, self-hosted \u2014 FerrFlow s\'adapte \u00e0 votre forge. Un outil, toutes les plateformes.',
    'features.commits.title': 'Conventional commits',
    'features.commits.desc': 'Lit votre historique de commits pour d\u00e9terminer les bumps automatiquement. Pas de maintenance manuelle de changelog.',
    'features.changelog.title': 'Z\u00e9ro infra',
    'features.changelog.desc': 'Un seul binaire, pas de daemon, pas de serveur, pas de DB. Tourne l\u00e0 o\u00f9 votre CI tourne.',
    'features.ci.title': 'Open source',
    'features.ci.desc': 'Enti\u00e8rement open source. Self-host, forkez, auditez \u2014 votre choix.',
    'features.private.title': 'Reste priv\u00e9',
    'features.private.desc': 'Votre code ne quitte jamais votre infrastructure. FerrFlow tourne dans votre propre pipeline.',

    // How it works
    'how.title': 'Comment \u00e7a marche',
    'how.step1.title': 'Installez le CLI',
    'how.step1.desc': 'Un binaire, toutes les fonctions. Linux, macOS et Windows.',
    'how.step2.title': 'Connectez votre projet',
    'how.step2.desc': 'FerrFlow d\u00e9tecte votre repo, votre forge et votre config automatiquement.',
    'how.step3.title': 'Livrez en confiance',
    'how.step3.desc': 'Bump versions, changelogs, tags et releases \u2014 en une seule commande.',

    // CTA
    'cta.title1': 'Pr\u00eat \u00e0 publier',
    'cta.title2': 'sans c\u00e9r\u00e9monie ?',
    'cta.subtitle': 'Gratuit et open source. Tourne dans votre propre CI. Aucune donn\u00e9e ne quitte votre infrastructure.',
    'cta.button': 'Voir sur GitHub',

    'prefooter.title': 'Pr\u00eat \u00e0 livrer.',
    'prefooter.subtitle': 'Sans le protocole.',

    // Footer
    'footer.rights': '\u00a9 2026 FerrLabs. Tous droits r\u00e9serv\u00e9s.',
    'footer.documentation': 'Documentation',
    'footer.releases': 'Releases',
    'footer.product': 'Produit',
    'footer.resources': 'Ressources',
    'footer.ferrlabs': 'FerrLabs',
    'footer.legal': 'L\u00e9gal',
    'footer.reportIssue': 'Signaler un probl\u00e8me',
    'footer.discussions': 'Discussions',
    'footer.terms': 'Conditions d\'utilisation',
    'footer.privacy': 'Politique de confidentialit\u00e9',

    // Landing page copy — rendered by src/components/LandingPage.astro on both / and /fr/.
    'cliPage.meta.title': 'FerrFlow CLI',
    'cliPage.meta.description': 'Un binaire Rust pour le versioning s\u00e9mantique automatis\u00e9, les changelogs et les releases. Monorepo-native, pilot\u00e9 par conventional commits, tourne dans toute CI.',
    'cliPage.hero.badge': 'Open source',
    'cliPage.hero.headline1': 'Publiez.',
    'cliPage.hero.headline2': 'Automatiquement.',
    'cliPage.hero.subheading': "Des conventional commits en entr\u00e9e, des versions s\u00e9mantiques en sortie. FerrFlow lit votre historique git, bump les versions dans tout votre monorepo, \u00e9crit les changelogs, tag et publie les releases GitHub \u2014 directement depuis votre CI. Juste du versioning, bien fait.",
    'cliPage.hero.download': 'T\u00e9l\u00e9charger',
    'cliPage.hero.docs': 'Documentation',
    'cliPage.features.title': 'Tout pour livrer vos releases',
    'cliPage.features.subtitle': 'Un seul binaire, z\u00e9ro plugin. Branchez sur n\'importe quelle CI et c\'est parti.',
    'cliPage.features.versioning.title': 'Versioning s\u00e9mantique',
    'cliPage.features.versioning.desc': 'Lit vos commits conventionnels et d\u00e9termine le bon bump. Strat\u00e9gies semver, calver, sequential et zerover support\u00e9es.',
    'cliPage.features.monorepo.title': 'Monorepo natif',
    'cliPage.features.monorepo.desc': 'Versionnez plusieurs packages ind\u00e9pendamment, avec chemins partag\u00e9s, cha\u00eenes de d\u00e9pendances et changelogs par package.',
    'cliPage.features.changelog.title': 'G\u00e9n\u00e9ration de changelog',
    'cliPage.features.changelog.desc': 'Changelogs structur\u00e9s et lisibles g\u00e9n\u00e9r\u00e9s depuis vos commits. Group\u00e9s par type, scop\u00e9s par package.',
    'cliPage.features.forge.title': 'Multi-forge',
    'cliPage.features.forge.desc': 'GitHub, GitLab, forges self-hosted. Tags, releases et commentaires de preview sur les PR \u2014 nativement support\u00e9s.',
    'cliPage.features.formats.title': '14+ formats',
    'cliPage.features.formats.desc': 'Cargo.toml, package.json, pyproject.toml, Chart.yaml, mix.exs, gemspec, pubspec.yaml, et plus \u2014 bump\u00e9s automatiquement.',
    'cliPage.terminal.scanning': 'Scan de 4 packages, 12 commits depuis la derni\u00e8re release...',
    'cliPage.terminal.bump': 'Bump ',
    'cliPage.terminal.tag': 'Tag  ',
    'cliPage.terminal.generating': 'G\u00e9n\u00e9ration des changelogs, tagging, push...',
    'cliPage.terminal.released': '✓ 2 releases GitHub publi\u00e9es',
    'cliPage.install.title': 'Install\u00e9 en quelques secondes',
    'cliPage.install.works': 'Fonctionne sur Linux, macOS et Windows. Aussi disponible en ',
    'cliPage.install.ghAction': 'GitHub Action',
    'cliPage.install.and': ' et ',
    'cliPage.install.glCi': 'template GitLab CI',
    'cliPage.install.dot': '.',


  },
} as const;

export function t(lang: Lang, key: keyof typeof ui.en): string {
  return ui[lang][key] ?? ui.en[key];
}
