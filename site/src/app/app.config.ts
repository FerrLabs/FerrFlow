import {
  ApplicationConfig,
  ErrorHandler,
  LOCALE_ID,
  provideZonelessChangeDetection,
} from '@angular/core';
import { provideHttpClient, withFetch, withInterceptors } from '@angular/common/http';
import { provideClientHydration } from '@angular/platform-browser';
import { Route } from '@angular/router';
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
import { TelemetryErrorHandler } from '@ferrlabs/telemetry/angular';

const changelogPostRoute: Route = {
  path: 'changelog/:slug',
  loadComponent: () => import('./pages/changelog/[slug].page'),
};

const localizedRoutes = LOCALES.map((locale) => ({
  path: locale,
  children: [changelogPostRoute, ...routes],
}));

export const appConfig: ApplicationConfig = {
  providers: [
    { provide: ErrorHandler, useClass: TelemetryErrorHandler },
    provideZonelessChangeDetection(),
    provideFileRouter(withExtraRoutes([...localizedRoutes, changelogPostRoute])),
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
