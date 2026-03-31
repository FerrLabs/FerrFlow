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
    'meta.title': 'FerrFlow — Automated Semantic Versioning',
    'meta.description': 'Universal semantic versioning for monorepos and classic repos',

    // Nav
    'nav.docs': 'Docs',
    'nav.playground': 'Playground',

    // Hero
    'hero.headline1': 'Release.',
    'hero.headline2': 'Automatically.',
    'hero.subheading': 'FerrFlow reads your commit history, picks the right version bump, writes your changelog, and publishes a tagged release — for any language, any repo layout.',
    'hero.cta.download': 'Download latest release',
    'hero.cta.docs': 'Read the docs',

    // Terminal
    'terminal.scanning': 'Scanning packages/api ...',

    // Metrics
    'metrics.title': 'Trusted in production',
    'metrics.subtitle': 'Real numbers from real teams shipping with FerrFlow.',
    'metrics.releases': 'Releases shipped',
    'metrics.repos': 'Repos tracked',
    'metrics.commits': 'Commits analyzed',

    // Features
    'features.title': 'Why FerrFlow?',
    'features.subtitle': 'Automate your release workflow with confidence.',
    'features.monorepo.title': 'Monorepo Support',
    'features.monorepo.desc': 'Version multiple packages independently or together. Each package follows its own cadence.',
    'features.agnostic.title': 'Language Agnostic',
    'features.agnostic.desc': 'JavaScript, Rust, Python, Go — configure any version file. FerrFlow handles the rest.',
    'features.commits.title': 'Conventional Commits',
    'features.commits.desc': 'Reads feat, fix, and breaking changes from your git history to determine the right bump.',
    'features.changelog.title': 'Changelog Generation',
    'features.changelog.desc': 'Beautiful, structured changelogs generated from your commits. Ready to publish.',
    'features.ci.title': 'CI/CD Native',
    'features.ci.desc': 'First-class GitHub Actions support. Works with GitLab CI, Jenkins, and anything else.',
    'features.private.title': 'Stays Private',
    'features.private.desc': 'Your code never leaves your infrastructure. Runs entirely inside your own pipeline.',

    // How it works
    'how.title': 'How it works',
    'how.step1.title': 'Push your commits',
    'how.step1.desc': "Write code, follow conventional commits. That's it.",
    'how.step2.title': 'FerrFlow runs',
    'how.step2.desc': 'Triggered by your CI, it scans commits since the last tag.',
    'how.step3.title': 'Release ships',
    'how.step3.desc': 'Version files updated, changelog written, tag created.',

    // CTA
    'cta.title1': 'Ready to ship',
    'cta.title2': 'without the ceremony?',
    'cta.subtitle': 'Free and open source. Runs in your own CI. No data leaves your infrastructure.',
    'cta.button': 'View on GitHub',

    // Footer
    'footer.rights': '© 2026 FerrFlow. All rights reserved.',
    'footer.documentation': 'Documentation',
    'footer.releases': 'Releases',
  },
  fr: {
    // Meta
    'meta.title': 'FerrFlow — Versionnage s\u00e9mantique automatis\u00e9',
    'meta.description': 'Versionnage s\u00e9mantique universel pour monorepos et repos classiques',

    // Nav
    'nav.docs': 'Docs',
    'nav.playground': 'Playground',

    // Hero
    'hero.headline1': 'Publiez.',
    'hero.headline2': 'Automatiquement.',
    'hero.subheading': "FerrFlow analyse votre historique de commits, d\u00e9termine le bon incr\u00e9ment de version, g\u00e9n\u00e8re votre changelog et publie une release tagg\u00e9e — quel que soit le langage ou la structure du repo.",
    'hero.cta.download': 'T\u00e9l\u00e9charger la derni\u00e8re version',
    'hero.cta.docs': 'Lire la documentation',

    // Terminal
    'terminal.scanning': 'Analyse de packages/api ...',

    // Metrics
    'metrics.title': 'Utilis\u00e9 en production',
    'metrics.subtitle': "Des chiffres concrets d'\u00e9quipes qui publient avec FerrFlow.",
    'metrics.releases': 'Releases publi\u00e9es',
    'metrics.repos': 'Repos suivis',
    'metrics.commits': 'Commits analys\u00e9s',

    // Features
    'features.title': 'Pourquoi FerrFlow ?',
    'features.subtitle': 'Automatisez vos releases en toute confiance.',
    'features.monorepo.title': 'Support Monorepo',
    'features.monorepo.desc': 'Versionnez plusieurs packages ind\u00e9pendamment ou ensemble. Chaque package suit sa propre cadence.',
    'features.agnostic.title': 'Agnostique au langage',
    'features.agnostic.desc': 'JavaScript, Rust, Python, Go — configurez n\'importe quel fichier de version. FerrFlow s\'occupe du reste.',
    'features.commits.title': 'Conventional Commits',
    'features.commits.desc': 'Analyse les feat, fix et breaking changes de votre historique git pour d\u00e9terminer le bon incr\u00e9ment.',
    'features.changelog.title': 'G\u00e9n\u00e9ration de changelog',
    'features.changelog.desc': 'Des changelogs structur\u00e9s et lisibles, g\u00e9n\u00e9r\u00e9s depuis vos commits. Pr\u00eats \u00e0 publier.',
    'features.ci.title': 'CI/CD natif',
    'features.ci.desc': 'Support GitHub Actions int\u00e9gr\u00e9. Compatible avec GitLab CI, Jenkins et tout le reste.',
    'features.private.title': 'Reste priv\u00e9',
    'features.private.desc': 'Votre code ne quitte jamais votre infrastructure. Tourne enti\u00e8rement dans votre propre pipeline.',

    // How it works
    'how.title': 'Comment \u00e7a marche',
    'how.step1.title': 'Poussez vos commits',
    'how.step1.desc': '\u00c9crivez du code, suivez les conventional commits. C\'est tout.',
    'how.step2.title': 'FerrFlow s\'ex\u00e9cute',
    'how.step2.desc': 'D\u00e9clench\u00e9 par votre CI, il analyse les commits depuis le dernier tag.',
    'how.step3.title': 'La release part',
    'how.step3.desc': 'Fichiers de version mis \u00e0 jour, changelog \u00e9crit, tag cr\u00e9\u00e9.',

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
