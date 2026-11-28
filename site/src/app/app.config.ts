import {
  ApplicationConfig,
  ErrorHandler,
  LOCALE_ID,
  inject,
  provideZonelessChangeDetection,
} from '@angular/core';
import { provideHttpClient, withFetch } from '@angular/common/http';
import { provideClientHydration } from '@angular/platform-browser';
import { ROUTES, Route } from '@angular/router';
import { provideContent, withMarkdownRenderer } from '@analogjs/content';
import { provideFileRouter, withExtraRoutes } from '@analogjs/router';
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

const localePaths: ReadonlySet<string | undefined> = new Set(LOCALES);

const isLocaleRoute = (route: Route): boolean => localePaths.has(route.path);

const localizedRoutes: Route[] = LOCALES.map((locale) => ({
  path: locale,
  loadChildren: () => {
    const siteRoutes = inject(ROUTES)
      .flat()
      .filter((route) => !isLocaleRoute(route));
    return [changelogPostRoute, ...siteRoutes.filter((route) => route !== changelogPostRoute)];
  },
}));

export const appConfig: ApplicationConfig = {
  providers: [
    { provide: ErrorHandler, useClass: TelemetryErrorHandler },
    provideZonelessChangeDetection(),
    provideFileRouter(withExtraRoutes([...localizedRoutes, changelogPostRoute])),
    provideClientHydration(),
    provideHttpClient(withFetch()),
    provideContent(withMarkdownRenderer()),
    provideI18n({
      loader: async (locale) => (await import(`../i18n/${locale}.json`)).default,
    }),
    { provide: LOCALE_ID, useFactory: () => injectLocale() ?? 'en' },
    provideSiteChrome(ferrflowChrome),
  ],
};
