import { ChangeDetectionStrategy, Component } from '@angular/core';
import { SiteShellComponent } from '@ferrlabs/ui-ng';
import { LandingComponent } from '../components/landing.component';

@Component({
  selector: 'flr-home',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SiteShellComponent, LandingComponent],
  template: `
    <flr-site-shell [title]="title" [description]="description">
      <flr-landing />
    </flr-site-shell>
  `,
})
export default class HomePage {
  protected readonly title = $localize`:@@meta.title:FerrFlow — Universal semantic versioning CLI`;
  protected readonly description = $localize`:@@meta.description:FerrFlow is an open-source CLI that bumps versions, generates changelogs, and ships releases — across 14+ file formats, monorepos, and CI. One Rust binary, any language, git-driven.`;
}
