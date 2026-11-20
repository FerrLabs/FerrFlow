import { benchmarks } from '../../data/benchmarks';
import { selectSpeedBenchmark, type SpeedBenchmark } from './speed-selection';

const SPEED_FIXTURE = 'mono-medium';
const SPEED_FIXTURE_SHAPE = { packages: 50, commitsPerPackage: 500 };

export function speedBenchmark(): SpeedBenchmark | undefined {
  return selectSpeedBenchmark(benchmarks.benchmarks, SPEED_FIXTURE, SPEED_FIXTURE_SHAPE);
}
