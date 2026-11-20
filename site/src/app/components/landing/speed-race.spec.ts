import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';
import { ferrflowBarPercent, formatMs, progressPercent, speedup } from './speed-race.ts';

describe('speed-race', () => {
  it('scales the ferrflow bar to its share of the semantic-release time', () => {
    assert.equal(ferrflowBarPercent(150, 1500), 10);
  });

  it('never lets the ferrflow bar fall under 1 percent', () => {
    assert.equal(ferrflowBarPercent(5, 1500), 1);
  });

  it('caps progress at 100 percent', () => {
    assert.equal(progressPercent(2000, 1500), 100);
    assert.equal(progressPercent(0, 1500), 0);
  });

  it('rounds the speedup factor', () => {
    assert.equal(speedup(7, 2100), 300);
    assert.equal(speedup(8, 2100), 263);
  });

  it('groups thousands in the readout for the given locale', () => {
    assert.equal(formatMs(2100, 'en'), '2,100 ms');
    assert.equal(formatMs(14.6, 'en'), '15 ms');
  });
});
