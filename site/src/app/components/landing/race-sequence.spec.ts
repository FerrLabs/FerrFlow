import { strict as assert } from 'node:assert';
import { afterEach, beforeEach, describe, it, mock } from 'node:test';
import {
  CHAR_DELAY_MS,
  IDLE_STATE,
  RACE_DELAY_MS,
  RaceSequence,
  START_DELAY_MS,
  type RaceState,
} from './race-sequence.ts';

const COMMAND = 'ferrflow release --dry-run';
const TIMINGS = { ferrflowMs: 7, competitorMs: 2100 };
const COMPETITOR_MS = 2100;
const FRAME_MS = 16;
const TYPING_END_MS = START_DELAY_MS + (COMMAND.length - 1) * CHAR_DELAY_MS;
const RACE_START_MS = TYPING_END_MS + RACE_DELAY_MS;

const advance = (ms: number): void => {
  for (let elapsed = 0; elapsed < ms; elapsed++) mock.timers.tick(1);
};

const globals = globalThis as unknown as Record<string, unknown>;

describe('RaceSequence', () => {
  let states: RaceState[];
  let sequence: RaceSequence;

  const latest = (): RaceState => states.at(-1) ?? IDLE_STATE;

  beforeEach(() => {
    mock.timers.enable({ apis: ['setTimeout', 'Date'] });
    globals['requestAnimationFrame'] = (callback: () => void) => setTimeout(callback, FRAME_MS);
    globals['cancelAnimationFrame'] = (handle: ReturnType<typeof setTimeout>) =>
      clearTimeout(handle);
    states = [];
    sequence = new RaceSequence(COMMAND, TIMINGS, (state) => states.push(state));
  });

  afterEach(() => {
    sequence.destroy();
    mock.timers.reset();
    delete globals['requestAnimationFrame'];
    delete globals['cancelAnimationFrame'];
  });

  it('types the command one character at a time after the initial delay', () => {
    sequence.start();
    advance(START_DELAY_MS - 1);
    assert.equal(latest().typed, '');
    advance(1);
    assert.equal(latest().typed, 'f');
    advance(CHAR_DELAY_MS);
    assert.equal(latest().typed, 'fe');
    advance(TYPING_END_MS - START_DELAY_MS - CHAR_DELAY_MS);
    assert.equal(latest().typed, COMMAND);
  });

  it('keeps the output hidden until the ferrflow time has elapsed', () => {
    sequence.start();
    advance(RACE_START_MS + TIMINGS.ferrflowMs - 1);
    assert.equal(latest().resultShown, false);
    advance(1);
    assert.equal(latest().resultShown, true);
    assert.equal(latest().finished, false);
  });

  it('ends the semantic-release readout exactly on its time', () => {
    sequence.start();
    advance(RACE_START_MS + COMPETITOR_MS - 1);
    assert.equal(latest().finished, false);
    assert.ok(latest().competitorMs < COMPETITOR_MS);
    advance(FRAME_MS);
    assert.equal(latest().competitorMs, COMPETITOR_MS);
    assert.equal(latest().finished, true);
  });

  it('stops emitting once the race is finished', () => {
    sequence.start();
    advance(RACE_START_MS + COMPETITOR_MS + FRAME_MS);
    const emitted = states.length;
    advance(5000);
    assert.equal(states.length, emitted);
  });

  it('resets to idle and runs again from the start on replay', () => {
    sequence.start();
    advance(RACE_START_MS + COMPETITOR_MS + FRAME_MS);
    assert.equal(latest().finished, true);
    sequence.start();
    assert.deepEqual(latest(), IDLE_STATE);
    advance(RACE_START_MS + TIMINGS.ferrflowMs);
    assert.equal(latest().resultShown, true);
    assert.equal(latest().finished, false);
    advance(COMPETITOR_MS - TIMINGS.ferrflowMs + FRAME_MS);
    assert.equal(latest().finished, true);
  });

  it('renders the final state immediately without timers', () => {
    sequence.showFinal();
    assert.deepEqual(latest(), {
      typed: COMMAND,
      resultShown: true,
      competitorMs: COMPETITOR_MS,
      finished: true,
    });
    const emitted = states.length;
    advance(5000);
    assert.equal(states.length, emitted);
  });

  it('clears pending timers and frames on destroy', () => {
    sequence.start();
    advance(RACE_START_MS + 100);
    const emitted = states.length;
    sequence.destroy();
    advance(5000);
    assert.equal(states.length, emitted);
  });

  it('finishes as soon as the output shows when there is no competitor', () => {
    const solo: RaceState[] = [];
    const soloSequence = new RaceSequence(
      COMMAND,
      { ferrflowMs: 7, competitorMs: undefined },
      (state) => solo.push(state),
    );
    soloSequence.start();
    advance(RACE_START_MS + 7);
    assert.equal(solo.at(-1)?.resultShown, true);
    assert.equal(solo.at(-1)?.finished, true);
    assert.equal(solo.at(-1)?.competitorMs, 0);
    soloSequence.destroy();
  });

  it('clears the typing timer on destroy', () => {
    sequence.start();
    advance(START_DELAY_MS);
    const emitted = states.length;
    sequence.destroy();
    advance(5000);
    assert.equal(states.length, emitted);
  });
});
