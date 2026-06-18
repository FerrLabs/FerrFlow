import { ChangeDetectionStrategy, Component } from '@angular/core';
import { ShellComponent } from '../../components/shell.component';
import { PerformanceComponent } from '../../components/performance.component';

@Component({
  selector: 'flr-performance-fr',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ShellComponent, PerformanceComponent],
  template: `
    <flr-shell
      lang="fr"
      title="Performance — FerrFlow"
      description="Comment FerrFlow se compare à semantic-release, changesets, standard-version et commit-and-tag-version sur des fixtures réelles. Latence médiane, RSS, taille d'install — mesurés via hyperfine en CI."
      canonical="https://ferrflow.com/fr/performance/"
      enHref="/performance/"
      frHref="/fr/performance/"
    >
      <flr-performance lang="fr" />
    </flr-shell>
  `,
})
export default class PerformanceFrPage {}
