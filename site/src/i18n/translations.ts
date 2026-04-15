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
    'meta.title': 'FerrFlow — Developer Platform',
    'meta.description': 'Versioning, issues, and automation for developer teams.',

    // Nav
    'nav.docs': 'Docs',
    // 'nav.playground': 'Playground', // disabled — playground is temporarily unavailable
    'nav.login': 'Log in',
    'nav.products.cli.desc': 'One binary for the entire FerrFlow platform',
    'nav.products.issues.desc': 'Issue tracking from terminal to dashboard',
    'nav.products.secrets.desc': 'Project-scoped secret management',

    // Hero
    'hero.headline1': 'Ship faster.',
    'hero.headline2': 'Stay organized.',
    'hero.subheading': 'FerrFlow is a developer platform that automates releases, tracks issues, and keeps your workflow in sync — from commit to deployment.',
    'hero.cta.download': 'Download latest release',
    'hero.cta.docs': 'Read the docs',

    // Products
    'products.title': 'One platform, multiple tools',
    'products.subtitle': 'Each tool works standalone. Together, they cover your entire release workflow.',
    'products.versioning.name': 'CLI',
    'products.versioning.desc': 'One binary for versioning, releases, and changelogs. Reads your commits, picks the right bump, and publishes to any forge.',
    'products.versioning.cta': 'Get started',
    'products.issues.name': 'Issues',
    'products.issues.desc': 'CLI-native issue tracking with a clean web dashboard. Create tickets from your terminal, triage from the browser, automate with AI.',
    'products.issues.cta': 'Learn more',
    'products.issues.badge': 'Coming soon',
    'products.secrets.name': 'Secrets',
    'products.secrets.desc': 'Project-scoped secret management with zero infra. Store, rotate, and inject secrets directly from your CLI into CI pipelines.',
    'products.secrets.cta': 'Learn more',
    'products.secrets.badge': 'Coming soon',
    'products.agent.name': 'Agent',
    'products.agent.desc': 'AI-powered automation and project insights. Create tickets from commits, classify issues, analyze velocity, and get actionable suggestions.',
    'products.agent.cta': 'Coming soon',
    'products.agent.badge': 'Coming soon',

    // Terminal
    'terminal.scanning': 'Scanning packages/api ...',

    // Features
    'features.title': 'Why FerrFlow?',
    'features.subtitle': 'A developer platform built for the way you actually work.',
    'features.monorepo.title': 'CLI-first',
    'features.monorepo.desc': 'Every feature starts in the terminal. Release, track issues, manage secrets — all from one CLI.',
    'features.agnostic.title': 'Multi-forge',
    'features.agnostic.desc': 'GitHub, GitLab, self-hosted — FerrFlow adapts to your forge. One tool, any platform.',
    'features.commits.title': 'AI-powered',
    'features.commits.desc': 'Classify issues, summarize sprints, suggest priorities. Intelligence built into the workflow.',
    'features.changelog.title': 'Zero infra',
    'features.changelog.desc': 'No servers to maintain. Secrets, issues, and releases are managed through the FerrFlow API.',
    'features.ci.title': 'Open source',
    'features.ci.desc': 'The CLI is fully open source. Self-host or use the hosted platform — your choice.',
    'features.private.title': 'Stays private',
    'features.private.desc': 'Your code never leaves your infrastructure. The CLI runs entirely inside your own pipeline.',

    // How it works
    'how.title': 'How it works',
    'how.step1.title': 'Install the CLI',
    'how.step1.desc': 'One binary, all features. Works on Linux, macOS, and Windows.',
    'how.step2.title': 'Connect your project',
    'how.step2.desc': 'FerrFlow detects your repo, forge, and config automatically.',
    'how.step3.title': 'Ship and track',
    'how.step3.desc': 'Releases, issues, secrets — managed from your terminal and dashboard.',

    // CTA
    'cta.title1': 'Ready to ship',
    'cta.title2': 'without the ceremony?',
    'cta.subtitle': 'Open source. Runs in your own CI. No data leaves your infrastructure.',
    'cta.button': 'View on GitHub',

    // Footer
    'footer.rights': '© 2026 FerrFlow. All rights reserved.',
    'footer.documentation': 'Documentation',
    'footer.releases': 'Releases',
  },
  fr: {
    // Meta
    'meta.title': 'FerrFlow — Plateforme D\u00e9veloppeur',
    'meta.description': 'Versioning, issues et automatisation pour les \u00e9quipes de d\u00e9veloppement.',

    // Nav
    'nav.docs': 'Docs',
    // 'nav.playground': 'Playground', // disabled — playground is temporarily unavailable
    'nav.login': 'Connexion',
    'nav.products.cli.desc': 'Un seul binaire pour toute la plateforme FerrFlow',
    'nav.products.issues.desc': 'Suivi de tickets du terminal au dashboard',
    'nav.products.secrets.desc': 'Gestion de secrets par projet',

    // Hero
    'hero.headline1': 'Livrez vite.',
    'hero.headline2': 'Restez organis\u00e9.',
    'hero.subheading': "FerrFlow est une plateforme d\u00e9veloppeur qui automatise les releases, suit les issues et synchronise votre workflow — du commit au d\u00e9ploiement.",
    'hero.cta.download': 'T\u00e9l\u00e9charger la derni\u00e8re version',
    'hero.cta.docs': 'Lire la documentation',

    // Products
    'products.title': 'Une plateforme, plusieurs outils',
    'products.subtitle': 'Chaque outil fonctionne seul. Ensemble, ils couvrent tout votre workflow de release.',
    'products.versioning.name': 'CLI',
    'products.versioning.desc': 'Un binaire pour le versioning, les releases et les changelogs. Analyse vos commits, d\u00e9termine le bon incr\u00e9ment et publie sur toute forge.',
    'products.versioning.cta': 'Commencer',
    'products.issues.name': 'Issues',
    'products.issues.desc': 'Suivi de tickets natif au CLI avec un dashboard web. Cr\u00e9ez des tickets depuis votre terminal, triez depuis le navigateur, automatisez avec l\'IA.',
    'products.issues.cta': 'Ouvrir le dashboard',
    'products.issues.badge': 'Preview',
    'products.secrets.name': 'Secrets',
    'products.secrets.desc': 'Gestion de secrets par projet, sans infra. Stockez, tournez et injectez vos secrets depuis le CLI directement dans vos pipelines CI.',
    'products.secrets.cta': 'Bient\u00f4t disponible',
    'products.secrets.badge': 'Bient\u00f4t',
    'products.agent.name': 'Agent',
    'products.agent.desc': 'Automatisation et analyse de projet par IA. Cr\u00e9ez des tickets depuis vos commits, classifiez les issues, analysez la v\u00e9locit\u00e9.',
    'products.agent.cta': 'Bient\u00f4t disponible',
    'products.agent.badge': 'Bient\u00f4t',

    // Terminal
    'terminal.scanning': 'Analyse de packages/api ...',

    // Features
    'features.title': 'Pourquoi FerrFlow ?',
    'features.subtitle': 'Une plateforme d\u00e9veloppeur con\u00e7ue pour votre fa\u00e7on de travailler.',
    'features.monorepo.title': 'CLI-first',
    'features.monorepo.desc': 'Tout commence dans le terminal. Releases, issues, secrets — depuis un seul CLI.',
    'features.agnostic.title': 'Multi-forge',
    'features.agnostic.desc': 'GitHub, GitLab, self-hosted — FerrFlow s\'adapte \u00e0 votre forge. Un outil, toutes les plateformes.',
    'features.commits.title': 'Propuls\u00e9 par l\'IA',
    'features.commits.desc': 'Classifiez les issues, r\u00e9sumez les sprints, sugg\u00e9rez les priorit\u00e9s. L\'intelligence int\u00e9gr\u00e9e au workflow.',
    'features.changelog.title': 'Z\u00e9ro infra',
    'features.changelog.desc': 'Pas de serveurs \u00e0 maintenir. Secrets, issues et releases sont g\u00e9r\u00e9s via l\'API FerrFlow.',
    'features.ci.title': 'Open source',
    'features.ci.desc': 'Le CLI est enti\u00e8rement open source. Self-host ou plateforme h\u00e9berg\u00e9e — votre choix.',
    'features.private.title': 'Reste priv\u00e9',
    'features.private.desc': 'Votre code ne quitte jamais votre infrastructure. Le CLI tourne dans votre propre pipeline.',

    // How it works
    'how.title': 'Comment \u00e7a marche',
    'how.step1.title': 'Installez le CLI',
    'how.step1.desc': 'Un binaire, toutes les fonctions. Linux, macOS et Windows.',
    'how.step2.title': 'Connectez votre projet',
    'how.step2.desc': 'FerrFlow d\u00e9tecte votre repo, votre forge et votre config automatiquement.',
    'how.step3.title': 'Livrez et suivez',
    'how.step3.desc': 'Releases, issues, secrets — g\u00e9r\u00e9s depuis votre terminal et dashboard.',

    // CTA
    'cta.title1': 'Pr\u00eat \u00e0 publier',
    'cta.title2': 'sans c\u00e9r\u00e9monie ?',
    'cta.subtitle': 'Gratuit et open source. Tourne dans votre propre CI. Aucune donn\u00e9e ne quitte votre infrastructure.',
    'cta.button': 'Voir sur GitHub',

    // Footer
    'footer.rights': '\u00a9 2026 FerrFlow. Tous droits r\u00e9serv\u00e9s.',
    'footer.documentation': 'Documentation',
    'footer.releases': 'Releases',
  },
} as const;

export function t(lang: Lang, key: keyof typeof ui.en): string {
  return ui[lang][key] ?? ui.en[key];
}
