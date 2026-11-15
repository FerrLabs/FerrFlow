import {
  afterNextRender,
  ChangeDetectionStrategy,
  Component,
  inject,
  LOCALE_ID,
  PLATFORM_ID,
  signal,
} from '@angular/core';
import { isPlatformBrowser } from '@angular/common';
import {
  CONTACT_FORM_LABELS_EN,
  CONTACT_FORM_LABELS_FR,
  ContactFormComponent,
  readContactPrefill,
  SiteShellComponent,
  type ContactPrefill,
} from '@ferrlabs/ui-ng';

const FERRLABS_HOSTS = [
  'ferrlabs.com',
  'ferrflow.com',
  'ferrvault.com',
  'ferrtrack.com',
  'ferrgrowth.com',
  'ferrfleet.com',
  'ferrlens.com',
  'ferrgames.com',
] as const;

@Component({
  selector: 'flr-contact',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SiteShellComponent, ContactFormComponent],
  templateUrl: './contact.page.html',
  styleUrl: './contact.page.css',
})
export default class ContactPage {
  protected readonly endpoint = 'https://api.ferrlabs.com/contact';
  protected readonly locale = inject(LOCALE_ID) === 'fr' ? 'fr' : 'en';
  protected readonly labels =
    this.locale === 'fr' ? CONTACT_FORM_LABELS_FR : CONTACT_FORM_LABELS_EN;
  protected readonly prefill = signal<ContactPrefill>({});
  private readonly platformId = inject(PLATFORM_ID);

  protected readonly title = $localize`:@@contact.metaTitle:Contact: FerrFlow`;
  protected readonly description = $localize`:@@contact.metaDescription:Write to the FerrFlow team: questions, billing, security reports or requests about your personal data.`;
  protected readonly heading = $localize`:@@contact.h1:Talk to the team`;
  protected readonly lead = $localize`:@@contact.lead:A question about FerrFlow, a billing issue or something about your data: write to us here and a person on the team will answer.`;
  protected readonly emailPrompt = $localize`:@@contact.emailPrompt:Prefer email? Write to`;

  constructor() {
    afterNextRender(() => {
      if (!isPlatformBrowser(this.platformId)) return;
      this.prefill.set(readContactPrefill(location.search, FERRLABS_HOSTS));
    });
  }
}
