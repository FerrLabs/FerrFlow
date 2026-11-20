import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';
import { selectSpeedBenchmark } from './speed-selection.ts';

const SHAPE = { packages: 12, commitsPerPackage: 340 };

const cells = (ferrflow: number, competitor?: number) => ({
  'fix-ferrflow-binary-release-dry-run': { median_ms: ferrflow },
  ...(competitor === undefined
    ? {}
    : { 'fix-semantic-release-npm-release-dry-run': { median_ms: competitor } }),
});

describe('selectSpeedBenchmark', () => {
  it('reads both medians from the fixture cells, rounded', () => {
    assert.deepEqual(selectSpeedBenchmark(cells(7.4, 2099.6), 'fix', SHAPE), {
      ...SHAPE,
      competitor: 'semantic-release',
      ferrflowMs: 7,
      competitorMs: 2100,
    });
  });

  it('drops the competitor when its cell is missing', () => {
    assert.equal(selectSpeedBenchmark(cells(7), 'fix', SHAPE)?.competitorMs, undefined);
  });

  it('drops the competitor when it is not slower', () => {
    assert.equal(selectSpeedBenchmark(cells(50, 50), 'fix', SHAPE)?.competitorMs, undefined);
    assert.equal(selectSpeedBenchmark(cells(50, 20), 'fix', SHAPE)?.competitorMs, undefined);
  });

  it('returns nothing without the ferrflow cell', () => {
    assert.equal(selectSpeedBenchmark({}, 'fix', SHAPE), undefined);
  });
});
