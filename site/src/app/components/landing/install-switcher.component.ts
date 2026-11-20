import { ChangeDetectionStrategy, Component, computed, input, signal } from '@angular/core';
import { CopyCommandComponent } from './copy-command.component';

interface InstallMethod {
  readonly id: string;
  readonly label: string;
  readonly command: string;
  readonly prompt: boolean;
}

function targetIndex(key: string, index: number, count: number): number | undefined {
  switch (key) {
    case 'ArrowRight':
      return (index + 1) % count;
    case 'ArrowLeft':
      return (index - 1 + count) % count;
    case 'Home':
      return 0;
    case 'End':
      return count - 1;
    default:
      return undefined;
  }
}

@Component({
  selector: 'flr-install-switcher',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [CopyCommandComponent],
  templateUrl: './install-switcher.component.html',
  styleUrl: './install-switcher.component.css',
})
export class InstallSwitcherComponent {
  readonly actionMajor = input.required<string>();

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

  protected onKeydown(event: KeyboardEvent, index: number): void {
    const methods = this.methods();
    const target = targetIndex(event.key, index, methods.length);
    if (target === undefined) return;
    event.preventDefault();
    this.selected.set(methods[target].id);
    const tablist = (event.currentTarget as HTMLElement).parentElement;
    tablist?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[target]?.focus();
  }
}
