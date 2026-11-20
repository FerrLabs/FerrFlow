import { ChangeDetectionStrategy, Component, input, signal } from '@angular/core';

const RESET_MS = 2000;

@Component({
  selector: 'flr-copy-command',
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './copy-command.component.html',
  styleUrl: './copy-command.component.css',
})
export class CopyCommandComponent {
  readonly command = input.required<string>();
  readonly prompt = input(true);

  protected readonly copied = signal(false);
  private resetTimer: ReturnType<typeof setTimeout> | undefined;

  protected async copy(): Promise<void> {
    await navigator.clipboard.writeText(this.command());
    this.copied.set(true);
    clearTimeout(this.resetTimer);
    this.resetTimer = setTimeout(() => this.copied.set(false), RESET_MS);
  }
}
