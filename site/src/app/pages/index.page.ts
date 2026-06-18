import { ChangeDetectionStrategy, Component } from '@angular/core';
import { ShellComponent } from '../components/shell.component';
import { LandingComponent } from '../components/landing.component';
import { t } from '../i18n/translations';

@Component({
  selector: 'flr-home-en',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ShellComponent, LandingComponent],
  template: `
    <flr-shell
      lang="en"
      [title]="title"
      [description]="description"
      canonical="https://ferrflow.com/"
      enHref="/"
      frHref="/fr/"
    >
      <flr-landing lang="en" />
    </flr-shell>
  `,
})
export default class HomeEnPage {
  protected readonly title = t('en', 'meta.title');
  protected readonly description = t('en', 'meta.description');
}
