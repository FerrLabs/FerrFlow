import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';
import { buildSeries, formatMs } from './perf-series.ts';

const cell = (median_ms: number, startup_ms?: number) => ({
  median_ms,
  stddev_ms: 0,
  memory_mb: 0,
  ...(startup_ms === undefined ? {} : { startup_ms }),
});

describe('formatMs', () => {
  it('renders null as an empty label', () => {
    assert.equal(formatMs(null), '');
  });

  it('rounds sub-second values to whole milliseconds', () => {
    assert.equal(formatMs(7.4), '7 ms');
    assert.equal(formatMs(999.4), '999 ms');
  });

  it('switches to seconds with two decimals from 1000 ms', () => {
    assert.equal(formatMs(1000), '1.00 s');
    assert.equal(formatMs(2099.6), '2.10 s');
  });
});

describe('buildSeries', () => {
  const cells = {
    'fix|ferrflow|check': cell(5),
    'fix|semantic-release|check': cell(2000, 1500),
    'fix|ferrflow|tag': cell(6),
    'other|ferrflow|check': cell(9),
  };

  it('groups medians by fixture, command and tool', () => {
    const { data } = buildSeries(
      cells,
      ['fix', 'other'],
      ['check', 'tag'],
      ['ferrflow', 'semantic-release'],
    );
    assert.deepEqual(data, {
      fix: {
        check: { ferrflow: 5, 'semantic-release': 2000 },
        tag: { ferrflow: 6 },
      },
      other: { check: { ferrflow: 9 } },
    });
  });

  it('only records startup for cells that report it', () => {
    const { startup } = buildSeries(
      cells,
      ['fix', 'other'],
      ['check', 'tag'],
      ['ferrflow', 'semantic-release'],
    );
    assert.deepEqual(startup, { fix: { check: { 'semantic-release': 1500 } } });
  });

  it('keeps a startup of zero', () => {
    const { startup } = buildSeries({ 'f|t|c': cell(1, 0) }, ['f'], ['c'], ['t']);
    assert.deepEqual(startup, { f: { c: { t: 0 } } });
  });

  it('omits fixtures and commands without any cell', () => {
    const { data, startup } = buildSeries(cells, ['missing'], ['check'], ['ferrflow']);
    assert.deepEqual(data, {});
    assert.deepEqual(startup, {});
  });
});
