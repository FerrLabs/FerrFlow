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
    'nav.signin': 'Sign in',
    'nav.signup': 'Sign up',
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
    'products.versioning.desc': 'One binary for the entire platform. Releases, issues, secrets, and automation — all from your terminal.',
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

    // CLI page
    'cli.meta.title': 'FerrFlow CLI',
    'cli.meta.description': 'One binary for automated semantic versioning, changelogs, issue tracking, and secret management.',
    'cli.hero.badge': 'Open source',
    'cli.hero.title1': 'One binary.',
    'cli.hero.title2': 'Every workflow.',
    'cli.hero.subtitle': 'Versioning, releases, changelogs, issue tracking, secrets, and AI automation. Install the FerrFlow CLI and manage your entire developer workflow from the terminal.',
    'cli.hero.cta.download': 'Download',
    'cli.hero.cta.docs': 'Documentation',
    'cli.terminal.scanning': 'Scanning packages/api ...',
    'cli.terminal.bump': 'Bump ',
    'cli.terminal.tag': 'Tag  ',
    'cli.terminal.issueCreated': ' Issue #43 created',
    'cli.terminal.injecting': 'Injecting 3 secrets...',
    'cli.terminal.testsPassed': ' All tests passed',
    'cli.features.title': 'Everything in one tool',
    'cli.features.subtitle': 'No plugins, no extensions. Every feature ships with the binary.',
    'cli.features.versioning.title': 'Semantic versioning',
    'cli.features.versioning.desc': 'Reads conventional commits and determines the right bump. Supports semver, calver, sequential, and zerover strategies.',
    'cli.features.monorepo.title': 'Monorepo native',
    'cli.features.monorepo.desc': 'Version multiple packages independently with shared paths, dependency chains, and per-package changelogs.',
    'cli.features.changelog.title': 'Changelog generation',
    'cli.features.changelog.desc': 'Structured, readable changelogs generated from your commits. Grouped by type, scoped per package.',
    'cli.features.issues.title': 'Issue tracking',
    'cli.features.issues.desc': 'Create, list, and manage issues from the terminal. Synced to the web dashboard at app.ferrflow.com.',
    'cli.features.secrets.title': 'Secret management',
    'cli.features.secrets.desc': 'Store, rotate, and inject secrets into CI pipelines. Encrypted at rest, project-scoped, no Vault needed.',
    'cli.features.ai.title': 'AI agent',
    'cli.features.ai.desc': 'Classify issues, summarize sprints, and suggest priorities. Built-in intelligence across every workflow.',
    'cli.install.title': 'Install in seconds',
    'cli.install.commentMac': '# macOS / Linux',
    'cli.install.commentNpm': '# npm',
    'cli.install.commentDocker': '# Docker',
    'cli.install.worksOn': 'Works on Linux, macOS, and Windows. Also available as a ',
    'cli.install.githubAction': 'GitHub Action',
    'cli.install.and': ' and ',
    'cli.install.gitlabCi': 'GitLab CI template',
    'cli.install.period': '.',
    'cli.cta.title': 'Start shipping',
    'cli.cta.subtitle': 'Open source. Works with GitHub, GitLab, and self-hosted forges.',
    'cli.cta.button': 'View on GitHub',

    // Issues product page — FR mirror lives at src/pages/fr/issues.astro.
    'issues.meta.title': 'FerrFlow Issues',
    'issues.meta.description': 'CLI-native issue tracking with a clean web dashboard. Coming soon.',
    'issues.hero.badge': 'Coming soon',
    'issues.hero.headline1': 'Issues that live',
    'issues.hero.headline2': 'where you work.',
    'issues.hero.subheading': 'Create tickets from your terminal, triage from a clean web dashboard, automate with AI. No more context switching between your editor and a browser tab.',
    'issues.preview.title': 'Coming soon',
    'issues.preview.subtitle': 'Under active development',
    'issues.features.title': 'What to expect',
    'issues.features.dashboard.title': 'Web dashboard',
    'issues.features.dashboard.desc': 'A fast, clean dashboard at app.ferrflow.com to view, filter, and manage issues across all your projects.',
    'issues.features.cli.title': 'CLI integration',
    'issues.features.cli.desc': 'ferrflow issues create, list, update, comment. Manage tickets without leaving your terminal.',
    'issues.features.ai.title': 'AI-powered triage',
    'issues.features.ai.desc': 'Automatic label classification, priority suggestions, and sprint summaries built into the workflow.',
    'issues.features.scoped.title': 'Project-scoped',
    'issues.features.scoped.desc': 'Issues tied to projects with role-based access. FerrFlow is the source of truth for your team.',
    'issues.cta.title1': 'Tired of switching tabs',
    'issues.cta.title2': 'to manage tickets?',
    'issues.cta.subtitle': 'FerrFlow Issues brings tracking to your terminal and your browser. One source of truth.',
    'issues.cta.button': 'View on GitHub',
  },
  fr: {
    // Meta
    'meta.title': 'FerrFlow — Plateforme D\u00e9veloppeur',
    'meta.description': 'Versioning, issues et automatisation pour les \u00e9quipes de d\u00e9veloppement.',

    // Nav
    'nav.docs': 'Docs',
    // 'nav.playground': 'Playground', // disabled — playground is temporarily unavailable
    'nav.signin': 'Connexion',
    'nav.signup': 'Inscription',
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
    'products.versioning.desc': 'Un seul binaire pour toute la plateforme. Releases, issues, secrets et automatisation — depuis votre terminal.',
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

    // CLI page
    'cli.meta.title': 'FerrFlow CLI',
    'cli.meta.description': 'Un seul binaire pour le versioning automatis\u00e9, les changelogs, le suivi d\'issues et la gestion des secrets.',
    'cli.hero.badge': 'Open source',
    'cli.hero.title1': 'Un binaire.',
    'cli.hero.title2': 'Tous vos workflows.',
    'cli.hero.subtitle': 'Versioning, releases, changelogs, suivi d\'issues, secrets et automatisation IA. Installez le CLI FerrFlow et pilotez tout votre workflow d\u00e9veloppeur depuis le terminal.',
    'cli.hero.cta.download': 'T\u00e9l\u00e9charger',
    'cli.hero.cta.docs': 'Documentation',
    'cli.terminal.scanning': 'Analyse de packages/api ...',
    'cli.terminal.bump': 'Bump ',
    'cli.terminal.tag': 'Tag  ',
    'cli.terminal.issueCreated': ' Issue #43 cr\u00e9\u00e9e',
    'cli.terminal.injecting': 'Injection de 3 secrets...',
    'cli.terminal.testsPassed': ' Tous les tests pass\u00e9s',
    'cli.features.title': 'Tout dans un seul outil',
    'cli.features.subtitle': 'Pas de plugins, pas d\'extensions. Chaque fonction est livr\u00e9e avec le binaire.',
    'cli.features.versioning.title': 'Versioning s\u00e9mantique',
    'cli.features.versioning.desc': 'Lit vos commits conventionnels et d\u00e9termine le bon incr\u00e9ment. Supporte les strat\u00e9gies semver, calver, s\u00e9quentielle et zerover.',
    'cli.features.monorepo.title': 'Natif monorepo',
    'cli.features.monorepo.desc': 'Versionnez plusieurs packages ind\u00e9pendamment avec chemins partag\u00e9s, cha\u00eenes de d\u00e9pendances et changelogs par package.',
    'cli.features.changelog.title': 'G\u00e9n\u00e9ration de changelog',
    'cli.features.changelog.desc': 'Des changelogs structur\u00e9s et lisibles g\u00e9n\u00e9r\u00e9s depuis vos commits. Group\u00e9s par type, scop\u00e9s par package.',
    'cli.features.issues.title': 'Suivi de tickets',
    'cli.features.issues.desc': 'Cr\u00e9ez, listez et g\u00e9rez vos tickets depuis le terminal. Synchronis\u00e9s avec le dashboard web sur app.ferrflow.com.',
    'cli.features.secrets.title': 'Gestion de secrets',
    'cli.features.secrets.desc': 'Stockez, tournez et injectez vos secrets dans vos pipelines CI. Chiffr\u00e9s au repos, scop\u00e9s par projet, sans Vault.',
    'cli.features.ai.title': 'Agent IA',
    'cli.features.ai.desc': 'Classifiez les tickets, r\u00e9sumez les sprints et sugg\u00e9rez les priorit\u00e9s. L\'intelligence int\u00e9gr\u00e9e \u00e0 chaque workflow.',
    'cli.install.title': 'Install\u00e9 en quelques secondes',
    'cli.install.commentMac': '# macOS / Linux',
    'cli.install.commentNpm': '# npm',
    'cli.install.commentDocker': '# Docker',
    'cli.install.worksOn': 'Fonctionne sur Linux, macOS et Windows. Aussi disponible en ',
    'cli.install.githubAction': 'GitHub Action',
    'cli.install.and': ' et ',
    'cli.install.gitlabCi': 'template GitLab CI',
    'cli.install.period': '.',
    'cli.cta.title': 'Commencez \u00e0 livrer',
    'cli.cta.subtitle': 'Open source. Compatible avec GitHub, GitLab et les forges self-hosted.',
    'cli.cta.button': 'Voir sur GitHub',

    // Issues product page — mirror at src/pages/fr/issues.astro.
    'issues.meta.title': 'FerrFlow Issues',
    'issues.meta.description': 'Suivi de tickets natif au CLI avec un dashboard web \u00e9pur\u00e9. Bient\u00f4t disponible.',
    'issues.hero.badge': 'Bient\u00f4t disponible',
    'issues.hero.headline1': 'Les tickets vivent',
    'issues.hero.headline2': 'l\u00e0 o\u00f9 vous travaillez.',
    'issues.hero.subheading': "Cr\u00e9ez des tickets depuis votre terminal, triez depuis un dashboard web \u00e9pur\u00e9, automatisez avec l'IA. Fini les allers-retours entre votre \u00e9diteur et un onglet navigateur.",
    'issues.preview.title': 'Bient\u00f4t disponible',
    'issues.preview.subtitle': 'En d\u00e9veloppement actif',
    'issues.features.title': 'Ce \u00e0 quoi vous attendre',
    'issues.features.dashboard.title': 'Dashboard web',
    'issues.features.dashboard.desc': 'Un dashboard rapide et \u00e9pur\u00e9 sur app.ferrflow.com pour visualiser, filtrer et g\u00e9rer vos tickets sur tous vos projets.',
    'issues.features.cli.title': 'Int\u00e9gration CLI',
    'issues.features.cli.desc': 'ferrflow issues create, list, update, comment. G\u00e9rez vos tickets sans quitter le terminal.',
    'issues.features.ai.title': 'Triage aliment\u00e9 par l\'IA',
    'issues.features.ai.desc': 'Classification automatique des labels, suggestions de priorit\u00e9s et r\u00e9sum\u00e9s de sprint int\u00e9gr\u00e9s au workflow.',
    'issues.features.scoped.title': 'Scop\u00e9 par projet',
    'issues.features.scoped.desc': "Tickets li\u00e9s aux projets avec acc\u00e8s par r\u00f4le. FerrFlow est la source de v\u00e9rit\u00e9 pour votre \u00e9quipe.",
    'issues.cta.title1': 'Marre de changer d\'onglet',
    'issues.cta.title2': 'pour g\u00e9rer les tickets\u00a0?',
    'issues.cta.subtitle': 'FerrFlow Issues ram\u00e8ne le suivi dans votre terminal et votre navigateur. Une seule source de v\u00e9rit\u00e9.',
    'issues.cta.button': 'Voir sur GitHub',
  },
} as const;

export function t(lang: Lang, key: keyof typeof ui.en): string {
  return ui[lang][key] ?? ui.en[key];
}
