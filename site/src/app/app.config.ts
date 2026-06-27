import { ApplicationConfig, LOCALE_ID, provideZonelessChangeDetection } from '@angular/core';
import { provideHttpClient, withFetch, withInterceptors } from '@angular/common/http';
import { provideClientHydration } from '@angular/platform-browser';
import { provideContent, withMarkdownRenderer } from '@analogjs/content';
import {
  provideFileRouter,
  requestContextInterceptor,
  routes,
  withExtraRoutes,
} from '@analogjs/router';
import { provideI18n } from '@analogjs/router/i18n';
import { injectLocale } from '@analogjs/router/tokens';
import { provideSiteChrome } from '@ferrlabs/ui-ng';
import { LOCALES } from './i18n/locales';
import { ferrflowChrome } from './site-chrome';

const localizedRoutes = LOCALES.map((locale) => ({ path: locale, children: routes }));

export const appConfig: ApplicationConfig = {
  providers: [
    provideZonelessChangeDetection(),
    provideFileRouter(withExtraRoutes(localizedRoutes)),
    provideClientHydration(),
    provideHttpClient(withFetch(), withInterceptors([requestContextInterceptor])),
    provideContent(withMarkdownRenderer()),
    provideI18n({
      loader: async (locale) => (await import(`../i18n/${locale}.json`)).default,
    }),
    { provide: LOCALE_ID, useFactory: () => injectLocale() ?? 'en' },
    provideSiteChrome(ferrflowChrome),
  ],
};
