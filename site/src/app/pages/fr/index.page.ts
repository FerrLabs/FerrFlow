import { ChangeDetectionStrategy, Component } from '@angular/core';
import { ShellComponent } from '../../components/shell.component';
import { LandingComponent } from '../../components/landing.component';
import { t } from '../../i18n/translations';

@Component({
  selector: 'flr-home-fr',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ShellComponent, LandingComponent],
  template: `
    <flr-shell
      lang="fr"
      [title]="title"
      [description]="description"
      canonical="https://ferrflow.com/fr/"
      enHref="/"
      frHref="/fr/"
    >
      <flr-landing lang="fr" />
    </flr-shell>
  `,
})
export default class HomeFrPage {
  protected readonly title = t('fr', 'meta.title');
  protected readonly description = t('fr', 'meta.description');
}
