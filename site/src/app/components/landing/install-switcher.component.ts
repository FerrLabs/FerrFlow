import { ChangeDetectionStrategy, Component, computed, input, signal } from '@angular/core';
import { TabComponent, TabsComponent } from '@ferrlabs/ui-ng';
import { CopyCommandComponent } from './copy-command.component';

interface InstallMethod {
  readonly id: string;
  readonly label: string;
  readonly command: string;
  readonly prompt: boolean;
}

@Component({
  selector: 'flr-install-switcher',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [CopyCommandComponent, TabsComponent, TabComponent],
  templateUrl: './install-switcher.component.html',
  styleUrl: './install-switcher.component.css',
})
export class InstallSwitcherComponent {
  readonly actionMajor = input.required<string>();

  protected readonly tabsLabel = $localize`:@@install.tabsLabel:Install method`;

  protected readonly methods = computed<readonly InstallMethod[]>(() => [
    { id: 'cargo', label: 'Cargo', command: 'cargo install ferrflow', prompt: true },
    { id: 'npm', label: 'npm', command: 'npm install -D ferrflow', prompt: true },
    {
      id: 'docker',
      label: 'Docker',
      command: 'docker run --rm -v $(pwd):/repo ghcr.io/ferrlabs/ferrflow:latest check',
      prompt: true,
    },
    {
      id: 'action',
      label: 'GitHub Actions',
      command: `- uses: FerrLabs/FerrFlow@${this.actionMajor()}\n  env:\n    GITHUB_TOKEN: \${{ secrets.GITHUB_TOKEN }}`,
      prompt: false,
    },
  ]);

  protected readonly selected = signal('cargo');

  protected select(id: string): void {
    this.selected.set(id);
  }
}
