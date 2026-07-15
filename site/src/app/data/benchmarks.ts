import rawBenchmarks from './benchmarks.json';

export interface BenchmarkCell {
  readonly median_ms: number;
  readonly stddev_ms: number;
  readonly memory_mb: string | number;
  readonly startup_ms?: number;
  readonly work_ms?: number;
}

export interface ParallelCell {
  readonly median_ms: number;
  readonly stddev_ms: number;
}

export interface InstallSize {
  readonly npm?: string;
  readonly binary?: string;
}

export interface Benchmarks {
  readonly timestamp: string;
  readonly ferrflow_version: string;
  readonly ferrflow_binary_size_mb: string;
  readonly ferrflow_npm_size_mb: string;
  readonly ferrflow_release_size_mb: string;
  readonly runner_cores?: number;
  readonly warmup?: number;
  readonly runs?: number;
  readonly benchmarks: Readonly<Record<string, BenchmarkCell>>;
  readonly install_sizes: Readonly<Record<string, InstallSize>>;
  readonly ferrflow_parallel?: Readonly<Record<string, ParallelCell>>;
  readonly ferrflow_cached?: Readonly<Record<string, ParallelCell>>;
}

export const benchmarks = rawBenchmarks as Benchmarks;
