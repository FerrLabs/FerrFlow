import { ApplicationConfig, provideZonelessChangeDetection } from '@angular/core';
import { provideHttpClient, withFetch, withInterceptors } from '@angular/common/http';
import { provideClientHydration } from '@angular/platform-browser';
import { provideContent, withMarkdownRenderer } from '@analogjs/content';
import { provideFileRouter, requestContextInterceptor } from '@analogjs/router';

export const appConfig: ApplicationConfig = {
  providers: [
    provideZonelessChangeDetection(),
    provideFileRouter(),
    provideClientHydration(),
    provideHttpClient(withFetch(), withInterceptors([requestContextInterceptor])),
    provideContent(withMarkdownRenderer()),
  ],
};
