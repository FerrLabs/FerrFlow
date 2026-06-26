export interface BenchmarkCell {
  readonly median_ms: number;
  readonly stddev_ms: number;
  readonly memory_mb: string | number;
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
  readonly benchmarks: Readonly<Record<string, BenchmarkCell>>;
  readonly install_sizes: Readonly<Record<string, InstallSize>>;
  readonly ferrflow_parallel?: Readonly<Record<string, ParallelCell>>;
}

export const benchmarks: Benchmarks = {
  timestamp: '2026-06-26T14:57:40Z',
  ferrflow_version: 'ferrflow 5.20.0',
  ferrflow_binary_size_mb: '10.0',
  ferrflow_npm_size_mb: '9.0',
  runner_cores: 4,
  benchmarks: {
    'mono-100-1k-changesets-npm-check': {
      median_ms: 842.3,
      stddev_ms: 27.1,
      memory_mb: '109.6',
    },
    'mono-100-1k-commit-and-tag-version-npm-check': {
      median_ms: 607.0,
      stddev_ms: 13.1,
      memory_mb: '90.6',
    },
    'mono-100-1k-ferrflow-binary-check': {
      median_ms: 3.4,
      stddev_ms: 1.3,
      memory_mb: '15.2',
    },
    'mono-100-1k-ferrflow-binary-release-dry-run': {
      median_ms: 83.3,
      stddev_ms: 8.3,
      memory_mb: '28.2',
    },
    'mono-100-1k-ferrflow-binary-tag': {
      median_ms: 20.8,
      stddev_ms: 0.9,
      memory_mb: '15.1',
    },
    'mono-100-1k-ferrflow-binary-validate': {
      median_ms: 3.5,
      stddev_ms: 0.1,
      memory_mb: '15.1',
    },
    'mono-100-1k-ferrflow-binary-version': {
      median_ms: 3.0,
      stddev_ms: 0.1,
      memory_mb: '13.2',
    },
    'mono-100-1k-ferrflow-docker-tag': {
      median_ms: 173.9,
      stddev_ms: 5.1,
      memory_mb: '45.9',
    },
    'mono-100-1k-ferrflow-docker-validate': {
      median_ms: 156.1,
      stddev_ms: 6.7,
      memory_mb: '42.8',
    },
    'mono-100-1k-ferrflow-docker-version': {
      median_ms: 155.1,
      stddev_ms: 10.4,
      memory_mb: '43.2',
    },
    'mono-100-1k-ferrflow-npm-check': {
      median_ms: 252.2,
      stddev_ms: 3.2,
      memory_mb: '91.4',
    },
    'mono-100-1k-ferrflow-npm-release-dry-run': {
      median_ms: 329.1,
      stddev_ms: 2.2,
      memory_mb: '91.5',
    },
    'mono-100-1k-ferrflow-npm-tag': {
      median_ms: 273.6,
      stddev_ms: 5.4,
      memory_mb: '92.0',
    },
    'mono-100-1k-ferrflow-npm-validate': {
      median_ms: 252.6,
      stddev_ms: 4.3,
      memory_mb: '91.6',
    },
    'mono-100-1k-ferrflow-npm-version': {
      median_ms: 248.9,
      stddev_ms: 3.3,
      memory_mb: '92.2',
    },
    'mono-100-1k-standard-version-npm-check': {
      median_ms: 634.3,
      stddev_ms: 13.8,
      memory_mb: '96.2',
    },
    'mono-50-5k-changesets-npm-check': {
      median_ms: 789.5,
      stddev_ms: 19.4,
      memory_mb: '110.8',
    },
    'mono-50-5k-commit-and-tag-version-npm-check': {
      median_ms: 1158.6,
      stddev_ms: 19.0,
      memory_mb: '117.5',
    },
    'mono-50-5k-ferrflow-binary-check': {
      median_ms: 3.2,
      stddev_ms: 0.3,
      memory_mb: '15.4',
    },
    'mono-50-5k-ferrflow-binary-release-dry-run': {
      median_ms: 366.5,
      stddev_ms: 9.7,
      memory_mb: '32.1',
    },
    'mono-50-5k-ferrflow-binary-tag': {
      median_ms: 75.1,
      stddev_ms: 2.2,
      memory_mb: '17.3',
    },
    'mono-50-5k-ferrflow-binary-validate': {
      median_ms: 3.5,
      stddev_ms: 0.3,
      memory_mb: '15.0',
    },
    'mono-50-5k-ferrflow-binary-version': {
      median_ms: 2.4,
      stddev_ms: 0.2,
      memory_mb: '13.2',
    },
    'mono-50-5k-ferrflow-docker-tag': {
      median_ms: 242.5,
      stddev_ms: 5.9,
      memory_mb: '43.3',
    },
    'mono-50-5k-ferrflow-docker-validate': {
      median_ms: 157.6,
      stddev_ms: 6.2,
      memory_mb: '45.4',
    },
    'mono-50-5k-ferrflow-docker-version': {
      median_ms: 156.6,
      stddev_ms: 6.3,
      memory_mb: '44.9',
    },
    'mono-50-5k-ferrflow-npm-check': {
      median_ms: 251.1,
      stddev_ms: 4.7,
      memory_mb: '91.9',
    },
    'mono-50-5k-ferrflow-npm-release-dry-run': {
      median_ms: 612.9,
      stddev_ms: 15.9,
      memory_mb: '91.2',
    },
    'mono-50-5k-ferrflow-npm-tag': {
      median_ms: 319.7,
      stddev_ms: 13.1,
      memory_mb: '92.2',
    },
    'mono-50-5k-ferrflow-npm-validate': {
      median_ms: 246.7,
      stddev_ms: 5.7,
      memory_mb: '91.7',
    },
    'mono-50-5k-ferrflow-npm-version': {
      median_ms: 250.8,
      stddev_ms: 4.7,
      memory_mb: '91.3',
    },
    'mono-50-5k-standard-version-npm-check': {
      median_ms: 1196.8,
      stddev_ms: 38.8,
      memory_mb: '125.8',
    },
    'mono-large-changesets-npm-check': {
      median_ms: 895.5,
      stddev_ms: 24.2,
      memory_mb: '113.6',
    },
    'mono-large-commit-and-tag-version-npm-check': {
      median_ms: 1851.0,
      stddev_ms: 149.1,
      memory_mb: '158.8',
    },
    'mono-large-ferrflow-binary-check': {
      median_ms: 4.2,
      stddev_ms: 0.2,
      memory_mb: '17.3',
    },
    'mono-large-ferrflow-binary-release-dry-run': {
      median_ms: 714.3,
      stddev_ms: 8.6,
      memory_mb: '36.0',
    },
    'mono-large-ferrflow-binary-tag': {
      median_ms: 148.6,
      stddev_ms: 3.5,
      memory_mb: '19.1',
    },
    'mono-large-ferrflow-binary-validate': {
      median_ms: 4.6,
      stddev_ms: 0.2,
      memory_mb: '17.1',
    },
    'mono-large-ferrflow-binary-version': {
      median_ms: 3.9,
      stddev_ms: 0.2,
      memory_mb: '15.2',
    },
    'mono-large-ferrflow-docker-tag': {
      median_ms: 336.6,
      stddev_ms: 9.1,
      memory_mb: '43.1',
    },
    'mono-large-ferrflow-docker-validate': {
      median_ms: 156.7,
      stddev_ms: 3.2,
      memory_mb: '42.8',
    },
    'mono-large-ferrflow-docker-version': {
      median_ms: 155.5,
      stddev_ms: 5.4,
      memory_mb: '43.9',
    },
    'mono-large-ferrflow-npm-check': {
      median_ms: 251.7,
      stddev_ms: 4.4,
      memory_mb: '92.0',
    },
    'mono-large-ferrflow-npm-release-dry-run': {
      median_ms: 984.3,
      stddev_ms: 13.7,
      memory_mb: '92.4',
    },
    'mono-large-ferrflow-npm-tag': {
      median_ms: 401.4,
      stddev_ms: 4.8,
      memory_mb: '91.6',
    },
    'mono-large-ferrflow-npm-validate': {
      median_ms: 246.9,
      stddev_ms: 4.9,
      memory_mb: '91.5',
    },
    'mono-large-ferrflow-npm-version': {
      median_ms: 245.6,
      stddev_ms: 4.3,
      memory_mb: '91.2',
    },
    'mono-large-standard-version-npm-check': {
      median_ms: 1888.7,
      stddev_ms: 21.4,
      memory_mb: '167.9',
    },
    'mono-medium-changesets-npm-check': {
      median_ms: 797.1,
      stddev_ms: 27.6,
      memory_mb: '109.4',
    },
    'mono-medium-commit-and-tag-version-npm-check': {
      median_ms: 539.3,
      stddev_ms: 8.4,
      memory_mb: '91.5',
    },
    'mono-medium-ferrflow-binary-check': {
      median_ms: 5.5,
      stddev_ms: 1.6,
      memory_mb: '15.4',
    },
    'mono-medium-ferrflow-binary-release-dry-run': {
      median_ms: 44.9,
      stddev_ms: 1.0,
      memory_mb: '28.2',
    },
    'mono-medium-ferrflow-binary-tag': {
      median_ms: 11.6,
      stddev_ms: 0.6,
      memory_mb: '15.3',
    },
    'mono-medium-ferrflow-binary-validate': {
      median_ms: 3.2,
      stddev_ms: 0.1,
      memory_mb: '15.1',
    },
    'mono-medium-ferrflow-binary-version': {
      median_ms: 2.6,
      stddev_ms: 0.2,
      memory_mb: '13.2',
    },
    'mono-medium-ferrflow-docker-tag': {
      median_ms: 167.1,
      stddev_ms: 2.8,
      memory_mb: '43.2',
    },
    'mono-medium-ferrflow-docker-validate': {
      median_ms: 152.4,
      stddev_ms: 6.1,
      memory_mb: '42.7',
    },
    'mono-medium-ferrflow-docker-version': {
      median_ms: 154.2,
      stddev_ms: 3.1,
      memory_mb: '43.1',
    },
    'mono-medium-ferrflow-npm-check': {
      median_ms: 252.4,
      stddev_ms: 3.7,
      memory_mb: '92.3',
    },
    'mono-medium-ferrflow-npm-release-dry-run': {
      median_ms: 289.3,
      stddev_ms: 3.2,
      memory_mb: '91.8',
    },
    'mono-medium-ferrflow-npm-tag': {
      median_ms: 254.7,
      stddev_ms: 4.0,
      memory_mb: '92.0',
    },
    'mono-medium-ferrflow-npm-validate': {
      median_ms: 247.8,
      stddev_ms: 4.0,
      memory_mb: '91.8',
    },
    'mono-medium-ferrflow-npm-version': {
      median_ms: 248.8,
      stddev_ms: 3.8,
      memory_mb: '91.6',
    },
    'mono-medium-standard-version-npm-check': {
      median_ms: 565.6,
      stddev_ms: 18.3,
      memory_mb: '94.7',
    },
    'mono-small-changesets-npm-check': {
      median_ms: 766.5,
      stddev_ms: 21.6,
      memory_mb: '106.3',
    },
    'mono-small-commit-and-tag-version-npm-check': {
      median_ms: 475.6,
      stddev_ms: 4.6,
      memory_mb: '91.3',
    },
    'mono-small-ferrflow-binary-check': {
      median_ms: 2.6,
      stddev_ms: 0.2,
      memory_mb: '13.2',
    },
    'mono-small-ferrflow-binary-release-dry-run': {
      median_ms: 16.2,
      stddev_ms: 0.4,
      memory_mb: '28.1',
    },
    'mono-small-ferrflow-binary-tag': {
      median_ms: 4.5,
      stddev_ms: 0.2,
      memory_mb: '13.3',
    },
    'mono-small-ferrflow-binary-validate': {
      median_ms: 2.8,
      stddev_ms: 2.8,
      memory_mb: '13.1',
    },
    'mono-small-ferrflow-binary-version': {
      median_ms: 2.6,
      stddev_ms: 0.5,
      memory_mb: '13.1',
    },
    'mono-small-ferrflow-docker-tag': {
      median_ms: 157.3,
      stddev_ms: 2.1,
      memory_mb: '45.5',
    },
    'mono-small-ferrflow-docker-validate': {
      median_ms: 151.2,
      stddev_ms: 3.1,
      memory_mb: '42.8',
    },
    'mono-small-ferrflow-docker-version': {
      median_ms: 153.5,
      stddev_ms: 5.0,
      memory_mb: '43.0',
    },
    'mono-small-ferrflow-npm-check': {
      median_ms: 250.7,
      stddev_ms: 4.9,
      memory_mb: '91.5',
    },
    'mono-small-ferrflow-npm-release-dry-run': {
      median_ms: 260.4,
      stddev_ms: 2.8,
      memory_mb: '91.6',
    },
    'mono-small-ferrflow-npm-tag': {
      median_ms: 246.8,
      stddev_ms: 5.9,
      memory_mb: '91.6',
    },
    'mono-small-ferrflow-npm-validate': {
      median_ms: 248.7,
      stddev_ms: 5.3,
      memory_mb: '92.1',
    },
    'mono-small-ferrflow-npm-version': {
      median_ms: 249.6,
      stddev_ms: 7.2,
      memory_mb: '91.4',
    },
    'mono-small-standard-version-npm-check': {
      median_ms: 480.1,
      stddev_ms: 7.7,
      memory_mb: '91.3',
    },
    'single-changesets-npm-check': {
      median_ms: 730.6,
      stddev_ms: 19.6,
      memory_mb: '103.7',
    },
    'single-commit-and-tag-version-npm-check': {
      median_ms: 468.4,
      stddev_ms: 10.1,
      memory_mb: '91.2',
    },
    'single-ferrflow-binary-check': {
      median_ms: 2.4,
      stddev_ms: 0.2,
      memory_mb: '13.2',
    },
    'single-ferrflow-binary-release-dry-run': {
      median_ms: 13.6,
      stddev_ms: 0.4,
      memory_mb: '20.0',
    },
    'single-ferrflow-binary-tag': {
      median_ms: 4.2,
      stddev_ms: 0.7,
      memory_mb: '13.2',
    },
    'single-ferrflow-binary-validate': {
      median_ms: 2.4,
      stddev_ms: 0.2,
      memory_mb: '13.0',
    },
    'single-ferrflow-binary-version': {
      median_ms: 2.6,
      stddev_ms: 0.1,
      memory_mb: '13.2',
    },
    'single-ferrflow-docker-tag': {
      median_ms: 159.6,
      stddev_ms: 7.9,
      memory_mb: '46.0',
    },
    'single-ferrflow-docker-validate': {
      median_ms: 153.4,
      stddev_ms: 4.9,
      memory_mb: '43.1',
    },
    'single-ferrflow-docker-version': {
      median_ms: 154.9,
      stddev_ms: 5.6,
      memory_mb: '43.4',
    },
    'single-ferrflow-npm-check': {
      median_ms: 253.5,
      stddev_ms: 10.1,
      memory_mb: '91.5',
    },
    'single-ferrflow-npm-release-dry-run': {
      median_ms: 265.1,
      stddev_ms: 3.7,
      memory_mb: '91.7',
    },
    'single-ferrflow-npm-tag': {
      median_ms: 266.1,
      stddev_ms: 12.9,
      memory_mb: '91.6',
    },
    'single-ferrflow-npm-validate': {
      median_ms: 254.0,
      stddev_ms: 3.6,
      memory_mb: '91.4',
    },
    'single-ferrflow-npm-version': {
      median_ms: 261.9,
      stddev_ms: 11.2,
      memory_mb: '92.3',
    },
    'single-standard-version-npm-check': {
      median_ms: 488.3,
      stddev_ms: 2.5,
      memory_mb: '91.8',
    },
  },
  ferrflow_parallel: {
    'mono-100-1k-check': {
      median_ms: 250.9,
      stddev_ms: 5.7,
    },
    'mono-100-1k-release-dry-run': {
      median_ms: 325.6,
      stddev_ms: 5.9,
    },
    'mono-100-1k-tag': {
      median_ms: 264.4,
      stddev_ms: 4.6,
    },
    'mono-100-1k-validate': {
      median_ms: 250.7,
      stddev_ms: 6.2,
    },
    'mono-100-1k-version': {
      median_ms: 252.9,
      stddev_ms: 6.8,
    },
    'mono-50-5k-check': {
      median_ms: 248.6,
      stddev_ms: 3.7,
    },
    'mono-50-5k-release-dry-run': {
      median_ms: 610.6,
      stddev_ms: 10.2,
    },
    'mono-50-5k-tag': {
      median_ms: 317.4,
      stddev_ms: 5.9,
    },
    'mono-50-5k-validate': {
      median_ms: 248.0,
      stddev_ms: 4.1,
    },
    'mono-50-5k-version': {
      median_ms: 255.1,
      stddev_ms: 4.5,
    },
    'mono-large-check': {
      median_ms: 249.4,
      stddev_ms: 2.7,
    },
    'mono-large-release-dry-run': {
      median_ms: 961.8,
      stddev_ms: 14.0,
    },
    'mono-large-tag': {
      median_ms: 386.0,
      stddev_ms: 7.8,
    },
    'mono-large-validate': {
      median_ms: 251.8,
      stddev_ms: 6.7,
    },
    'mono-large-version': {
      median_ms: 249.0,
      stddev_ms: 12.8,
    },
    'mono-medium-check': {
      median_ms: 248.9,
      stddev_ms: 4.6,
    },
    'mono-medium-release-dry-run': {
      median_ms: 287.8,
      stddev_ms: 8.2,
    },
    'mono-medium-tag': {
      median_ms: 254.2,
      stddev_ms: 3.2,
    },
    'mono-medium-validate': {
      median_ms: 247.2,
      stddev_ms: 5.4,
    },
    'mono-medium-version': {
      median_ms: 246.1,
      stddev_ms: 3.1,
    },
    'mono-small-check': {
      median_ms: 248.5,
      stddev_ms: 4.3,
    },
    'mono-small-release-dry-run': {
      median_ms: 262.4,
      stddev_ms: 5.6,
    },
    'mono-small-tag': {
      median_ms: 254.1,
      stddev_ms: 7.0,
    },
    'mono-small-validate': {
      median_ms: 246.2,
      stddev_ms: 2.3,
    },
    'mono-small-version': {
      median_ms: 249.6,
      stddev_ms: 8.0,
    },
    'single-check': {
      median_ms: 249.5,
      stddev_ms: 3.6,
    },
    'single-release-dry-run': {
      median_ms: 262.7,
      stddev_ms: 3.2,
    },
    'single-tag': {
      median_ms: 254.8,
      stddev_ms: 5.8,
    },
    'single-validate': {
      median_ms: 259.6,
      stddev_ms: 8.4,
    },
    'single-version': {
      median_ms: 262.9,
      stddev_ms: 13.0,
    },
  },
  install_sizes: {
    changesets: {
      npm: '26.0',
    },
    'commit-and-tag-version': {
      npm: '20.0',
    },
    ferrflow: {
      binary: '10.0',
      npm: '9.0',
    },
    'release-please': {
      npm: '95.0',
    },
    'semantic-release': {
      npm: '69.0',
    },
    'standard-version': {
      npm: '21.0',
    },
  },
  ferrflow_release_size_mb: '4.0',
};
