import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  inject,
  input,
  signal,
} from '@angular/core';

const RESET_MS = 2000;

type CopyStatus = 'idle' | 'copied' | 'failed';

@Component({
  selector: 'flr-copy-command',
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './copy-command.component.html',
  styleUrl: './copy-command.component.css',
})
export class CopyCommandComponent {
  readonly command = input.required<string>();
  readonly prompt = input(true);

  protected readonly status = signal<CopyStatus>('idle');
  private resetTimer: ReturnType<typeof setTimeout> | undefined;

  constructor() {
    inject(DestroyRef).onDestroy(() => clearTimeout(this.resetTimer));
  }

  protected async copy(): Promise<void> {
    try {
      await navigator.clipboard.writeText(this.command());
      this.show('copied');
    } catch {
      this.show('failed');
    }
  }

  private show(status: CopyStatus): void {
    this.status.set(status);
    clearTimeout(this.resetTimer);
    this.resetTimer = setTimeout(() => this.status.set('idle'), RESET_MS);
  }
}
