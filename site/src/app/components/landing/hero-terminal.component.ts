import {
  afterNextRender,
  ChangeDetectionStrategy,
  Component,
  computed,
  DestroyRef,
  ElementRef,
  inject,
  input,
  LOCALE_ID,
  signal,
} from '@angular/core';
import { resolveLocale } from '@ferrlabs/ui-ng';
import { IDLE_STATE, RaceSequence, type RaceState } from './race-sequence';
import type { SpeedBenchmark } from './speed-selection';
import { ferrflowBarPercent, formatMs, progressPercent, speedup } from './speed-race';

const COMMAND = 'ferrflow release --dry-run';
const SHOWN_PACKAGES = 3;
const REDUCED_MOTION_QUERY = '(prefers-reduced-motion: reduce)';

@Component({
  selector: 'flr-hero-terminal',
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './hero-terminal.component.html',
  styleUrl: './hero-terminal.component.css',
})
export class HeroTerminalComponent {
  readonly benchmark = input.required<SpeedBenchmark>();

  protected readonly state = signal<RaceState>(IDLE_STATE);
  protected readonly reducedMotion = signal(false);

  private readonly locale = resolveLocale(inject(LOCALE_ID));
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private sequence: RaceSequence | undefined;
  private observer: IntersectionObserver | undefined;

  protected readonly packages = computed(() => this.benchmark().packages);
  protected readonly commits = computed(() => {
    const { packages, commitsPerPackage } = this.benchmark();
    return new Intl.NumberFormat(this.locale).format(packages * commitsPerPackage);
  });
  protected readonly morePackages = computed(() => this.benchmark().packages - SHOWN_PACKAGES);
  protected readonly hasComparison = computed(() => this.benchmark().competitorMs !== undefined);

  protected readonly ferrflowReadout = computed(() =>
    formatMs(this.state().resultShown ? this.benchmark().ferrflowMs : 0, this.locale),
  );
  protected readonly competitorReadout = computed(() =>
    formatMs(this.state().competitorMs, this.locale),
  );
  protected readonly ferrflowBar = computed(() => {
    const { ferrflowMs, competitorMs } = this.benchmark();
    if (!this.state().resultShown || competitorMs === undefined) return 0;
    return ferrflowBarPercent(ferrflowMs, competitorMs);
  });
  protected readonly competitorBar = computed(() => {
    const { competitorMs } = this.benchmark();
    return competitorMs === undefined
      ? 0
      : progressPercent(this.state().competitorMs, competitorMs);
  });
  protected readonly speedup = computed(() => {
    const { ferrflowMs, competitorMs } = this.benchmark();
    return competitorMs === undefined ? 0 : speedup(ferrflowMs, competitorMs);
  });
  protected readonly label = computed(() => {
    const { packages, ferrflowMs, competitor, competitorMs } = this.benchmark();
    const ours = formatMs(ferrflowMs, this.locale);
    if (competitorMs === undefined) {
      return $localize`:@@terminal.labelSolo:ferrflow release dry run on ${packages}:PACKAGES: packages finishes in ${ours}:OURS:`;
    }
    const theirs = formatMs(competitorMs, this.locale);
    return $localize`:@@terminal.label:ferrflow release dry run on ${packages}:PACKAGES: packages finishes in ${ours}:OURS:, ${competitor}:COMPETITOR: takes ${theirs}:THEIRS:`;
  });
  protected readonly doneBadge = computed(() => {
    const ours = formatMs(this.benchmark().ferrflowMs, this.locale);
    return $localize`:@@terminal.done:done in ${ours}:OURS:`;
  });

  constructor() {
    afterNextRender(() => this.init());
    inject(DestroyRef).onDestroy(() => {
      this.observer?.disconnect();
      this.sequence?.destroy();
    });
  }

  protected replay(): void {
    this.sequence?.start();
  }

  private init(): void {
    const { ferrflowMs, competitorMs } = this.benchmark();
    this.sequence = new RaceSequence(COMMAND, { ferrflowMs, competitorMs }, (state) =>
      this.state.set(state),
    );
    if (matchMedia(REDUCED_MOTION_QUERY).matches) {
      this.reducedMotion.set(true);
      this.sequence.showFinal();
      return;
    }
    this.observer = new IntersectionObserver((entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      this.observer?.disconnect();
      this.sequence?.start();
    });
    this.observer.observe(this.host.nativeElement);
  }
}
