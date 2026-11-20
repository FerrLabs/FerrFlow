export const START_DELAY_MS = 300;
export const CHAR_DELAY_MS = 40;
export const RACE_DELAY_MS = 350;

export interface RaceTimings {
  readonly ferrflowMs: number;
  readonly competitorMs: number | undefined;
}

export interface RaceState {
  readonly typed: string;
  readonly resultShown: boolean;
  readonly competitorMs: number;
  readonly finished: boolean;
}

export const IDLE_STATE: RaceState = {
  typed: '',
  resultShown: false,
  competitorMs: 0,
  finished: false,
};

export class RaceSequence {
  private readonly command: string;
  private readonly timings: RaceTimings;
  private readonly emit: (state: RaceState) => void;
  private state: RaceState = IDLE_STATE;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private frame: number | undefined;

  constructor(command: string, timings: RaceTimings, emit: (state: RaceState) => void) {
    this.command = command;
    this.timings = timings;
    this.emit = emit;
  }

  start(): void {
    this.reset();
    this.timer = setTimeout(() => this.type(1), START_DELAY_MS);
  }

  reset(): void {
    this.cancel();
    this.update(IDLE_STATE);
  }

  showFinal(): void {
    this.cancel();
    this.update({
      typed: this.command,
      resultShown: true,
      competitorMs: this.timings.competitorMs ?? 0,
      finished: true,
    });
  }

  destroy(): void {
    this.cancel();
  }

  private type(length: number): void {
    this.update({ ...this.state, typed: this.command.slice(0, length) });
    if (length < this.command.length) {
      this.timer = setTimeout(() => this.type(length + 1), CHAR_DELAY_MS);
    } else {
      this.timer = setTimeout(() => this.race(), RACE_DELAY_MS);
    }
  }

  private race(): void {
    const { ferrflowMs, competitorMs } = this.timings;
    const startedAt = Date.now();
    this.timer = setTimeout(
      () => this.update({ ...this.state, resultShown: true, finished: competitorMs === undefined }),
      ferrflowMs,
    );
    if (competitorMs === undefined) return;
    const tick = (): void => {
      const elapsed = Math.min(Date.now() - startedAt, competitorMs);
      const finished = elapsed >= competitorMs;
      this.update({ ...this.state, competitorMs: elapsed, finished });
      this.frame = finished ? undefined : requestAnimationFrame(tick);
    };
    this.frame = requestAnimationFrame(tick);
  }

  private update(state: RaceState): void {
    this.state = state;
    this.emit(state);
  }

  private cancel(): void {
    clearTimeout(this.timer);
    this.timer = undefined;
    if (this.frame !== undefined) cancelAnimationFrame(this.frame);
    this.frame = undefined;
  }
}
