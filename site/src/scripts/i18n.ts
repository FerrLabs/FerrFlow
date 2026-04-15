const STORAGE_KEY = 'ferrflow-lang';

function isOnFr(): boolean {
  const path = window.location.pathname;
  return path === '/fr' || path === '/fr/' || path.startsWith('/fr/');
}

function isDocsPage(): boolean {
  const path = window.location.pathname;
  return path.startsWith('/docs') || /^\/fr\/docs/.test(path);
}

function getCurrentLang(): 'en' | 'fr' {
  return isOnFr() ? 'fr' : 'en';
}

function getSavedLang(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function saveLang(lang: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, lang);
  } catch {
    // localStorage unavailable
  }
}

function redirectToLang(lang: string): void {
  const path = window.location.pathname;

  if (lang === 'fr' && !isOnFr()) {
    // Only redirect to /fr/ for the home page (other pages don't have fr versions yet)
    if (path === '/' || path === '') {
      window.location.replace('/fr/');
    }
  } else if (lang === 'en' && isOnFr()) {
    const newPath = path.replace(/^\/fr\/?/, '/') || '/';
    window.location.replace(newPath);
  }
}

// On page load: detect or apply saved language
const saved = getSavedLang();
const current = getCurrentLang();

if (!saved) {
  // First visit: detect from browser language
  const detected = navigator.language.startsWith('fr') ? 'fr' : 'en';
  saveLang(detected);
  if (detected !== current && !isDocsPage()) {
    redirectToLang(detected);
  }
} else if (saved !== current && !isDocsPage()) {
  redirectToLang(saved);
}

// Expose for the language switcher
(window as any).__setLang = (lang: string) => {
  saveLang(lang);
  redirectToLang(lang);
};
