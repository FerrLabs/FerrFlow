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
  timestamp: '2026-06-17T11:10:58Z',
  ferrflow_version: 'ferrflow 5.15.1',
  ferrflow_binary_size_mb: '9.0',
  ferrflow_npm_size_mb: '13.0',
  ferrflow_release_size_mb: '3.8',
  benchmarks: {
    'mono-100-1k-changesets-npm-check': { median_ms: 865.7, stddev_ms: 15.3, memory_mb: '108.8' },
    'mono-100-1k-commit-and-tag-version-npm-check': {
      median_ms: 680.4,
      stddev_ms: 11.6,
      memory_mb: '89.2',
    },
    'mono-100-1k-ferrflow-binary-check': { median_ms: 3.4, stddev_ms: 0.1, memory_mb: '12.6' },
    'mono-100-1k-ferrflow-binary-release-dry-run': {
      median_ms: 113.0,
      stddev_ms: 0.8,
      memory_mb: '31.9',
    },
    'mono-100-1k-ferrflow-binary-tag': { median_ms: 27.2, stddev_ms: 0.6, memory_mb: '12.7' },
    'mono-100-1k-ferrflow-binary-validate': { median_ms: 3.4, stddev_ms: 0.2, memory_mb: '12.5' },
    'mono-100-1k-ferrflow-binary-version': { median_ms: 3.0, stddev_ms: 0.1, memory_mb: '12.5' },
    'mono-100-1k-ferrflow-docker-tag': { median_ms: 221.4, stddev_ms: 9.2, memory_mb: '44.3' },
    'mono-100-1k-ferrflow-docker-validate': { median_ms: 186.1, stddev_ms: 3.4, memory_mb: '44.3' },
    'mono-100-1k-ferrflow-docker-version': { median_ms: 185.7, stddev_ms: 7.1, memory_mb: '44.0' },
    'mono-100-1k-ferrflow-npm-check': { median_ms: 339.8, stddev_ms: 4.7, memory_mb: '89.3' },
    'mono-100-1k-ferrflow-npm-release-dry-run': {
      median_ms: 335.9,
      stddev_ms: 1.4,
      memory_mb: '89.4',
    },
    'mono-100-1k-ferrflow-npm-tag': { median_ms: 589.7, stddev_ms: 4.1, memory_mb: '89.1' },
    'mono-100-1k-ferrflow-npm-validate': { median_ms: 287.9, stddev_ms: 2.1, memory_mb: '89.1' },
    'mono-100-1k-ferrflow-npm-version': { median_ms: 290.6, stddev_ms: 1.6, memory_mb: '89.5' },
    'mono-100-1k-standard-version-npm-check': {
      median_ms: 695.9,
      stddev_ms: 12.6,
      memory_mb: '96.9',
    },
    'mono-50-5k-changesets-npm-check': { median_ms: 814.0, stddev_ms: 17.4, memory_mb: '110.9' },
    'mono-50-5k-commit-and-tag-version-npm-check': {
      median_ms: 1261.9,
      stddev_ms: 19.1,
      memory_mb: '117.5',
    },
    'mono-50-5k-ferrflow-binary-check': { median_ms: 2.9, stddev_ms: 0.1, memory_mb: '12.7' },
    'mono-50-5k-ferrflow-binary-release-dry-run': {
      median_ms: 512.8,
      stddev_ms: 4.4,
      memory_mb: '37.7',
    },
    'mono-50-5k-ferrflow-binary-tag': { median_ms: 103.9, stddev_ms: 2.1, memory_mb: '14.5' },
    'mono-50-5k-ferrflow-binary-validate': { median_ms: 2.8, stddev_ms: 0.0, memory_mb: '12.5' },
    'mono-50-5k-ferrflow-binary-version': { median_ms: 2.7, stddev_ms: 0.1, memory_mb: '12.5' },
    'mono-50-5k-ferrflow-docker-tag': { median_ms: 319.8, stddev_ms: 4.7, memory_mb: '43.8' },
    'mono-50-5k-ferrflow-docker-validate': { median_ms: 185.2, stddev_ms: 3.8, memory_mb: '44.8' },
    'mono-50-5k-ferrflow-docker-version': { median_ms: 186.2, stddev_ms: 4.4, memory_mb: '43.7' },
    'mono-50-5k-ferrflow-npm-check': { median_ms: 472.6, stddev_ms: 9.6, memory_mb: '89.3' },
    'mono-50-5k-ferrflow-npm-release-dry-run': {
      median_ms: 476.4,
      stddev_ms: 8.9,
      memory_mb: '89.1',
    },
    'mono-50-5k-ferrflow-npm-tag': { median_ms: 699.3, stddev_ms: 2.8, memory_mb: '89.1' },
    'mono-50-5k-ferrflow-npm-validate': { median_ms: 296.6, stddev_ms: 12.3, memory_mb: '89.3' },
    'mono-50-5k-ferrflow-npm-version': { median_ms: 291.7, stddev_ms: 5.9, memory_mb: '89.4' },
    'mono-50-5k-standard-version-npm-check': {
      median_ms: 1301.8,
      stddev_ms: 14.8,
      memory_mb: '126.6',
    },
    'mono-large-changesets-npm-check': { median_ms: 947.8, stddev_ms: 12.2, memory_mb: '112.5' },
    'mono-large-commit-and-tag-version-npm-check': {
      median_ms: 2011.2,
      stddev_ms: 37.2,
      memory_mb: '155.8',
    },
    'mono-large-ferrflow-binary-check': { median_ms: 4.2, stddev_ms: 0.1, memory_mb: '14.6' },
    'mono-large-ferrflow-binary-release-dry-run': {
      median_ms: 1035.3,
      stddev_ms: 5.8,
      memory_mb: '37.6',
    },
    'mono-large-ferrflow-binary-tag': { median_ms: 218.4, stddev_ms: 8.3, memory_mb: '18.7' },
    'mono-large-ferrflow-binary-validate': { median_ms: 4.6, stddev_ms: 0.1, memory_mb: '12.4' },
    'mono-large-ferrflow-binary-version': { median_ms: 3.9, stddev_ms: 0.1, memory_mb: '12.5' },
    'mono-large-ferrflow-docker-tag': { median_ms: 459.6, stddev_ms: 7.9, memory_mb: '45.7' },
    'mono-large-ferrflow-docker-validate': { median_ms: 185.9, stddev_ms: 5.8, memory_mb: '45.6' },
    'mono-large-ferrflow-docker-version': { median_ms: 186.8, stddev_ms: 5.1, memory_mb: '43.2' },
    'mono-large-ferrflow-npm-check': { median_ms: 680.6, stddev_ms: 13.8, memory_mb: '89.4' },
    'mono-large-ferrflow-npm-release-dry-run': {
      median_ms: 679.3,
      stddev_ms: 5.5,
      memory_mb: '89.2',
    },
    'mono-large-ferrflow-npm-tag': { median_ms: 2881.3, stddev_ms: 21.7, memory_mb: '89.5' },
    'mono-large-ferrflow-npm-validate': { median_ms: 296.9, stddev_ms: 1.9, memory_mb: '89.0' },
    'mono-large-ferrflow-npm-version': { median_ms: 300.4, stddev_ms: 4.8, memory_mb: '89.4' },
    'mono-large-standard-version-npm-check': {
      median_ms: 2029.8,
      stddev_ms: 32.0,
      memory_mb: '163.2',
    },
    'mono-medium-changesets-npm-check': { median_ms: 816.1, stddev_ms: 13.5, memory_mb: '110.1' },
    'mono-medium-commit-and-tag-version-npm-check': {
      median_ms: 597.2,
      stddev_ms: 10.0,
      memory_mb: '89.4',
    },
    'mono-medium-ferrflow-binary-check': { median_ms: 2.8, stddev_ms: 0.1, memory_mb: '12.7' },
    'mono-medium-ferrflow-binary-release-dry-run': {
      median_ms: 60.0,
      stddev_ms: 0.5,
      memory_mb: '31.7',
    },
    'mono-medium-ferrflow-binary-tag': { median_ms: 14.4, stddev_ms: 0.4, memory_mb: '12.7' },
    'mono-medium-ferrflow-binary-validate': { median_ms: 2.8, stddev_ms: 0.1, memory_mb: '12.4' },
    'mono-medium-ferrflow-binary-version': { median_ms: 2.6, stddev_ms: 0.1, memory_mb: '12.5' },
    'mono-medium-ferrflow-docker-tag': { median_ms: 199.7, stddev_ms: 5.0, memory_mb: '45.9' },
    'mono-medium-ferrflow-docker-validate': { median_ms: 186.4, stddev_ms: 3.2, memory_mb: '43.1' },
    'mono-medium-ferrflow-docker-version': { median_ms: 185.2, stddev_ms: 3.1, memory_mb: '44.2' },
    'mono-medium-ferrflow-npm-check': { median_ms: 319.4, stddev_ms: 2.7, memory_mb: '89.2' },
    'mono-medium-ferrflow-npm-release-dry-run': {
      median_ms: 316.7,
      stddev_ms: 1.4,
      memory_mb: '89.5',
    },
    'mono-medium-ferrflow-npm-tag': { median_ms: 375.5, stddev_ms: 2.2, memory_mb: '89.2' },
    'mono-medium-ferrflow-npm-validate': { median_ms: 289.2, stddev_ms: 16.2, memory_mb: '89.2' },
    'mono-medium-ferrflow-npm-version': { median_ms: 289.0, stddev_ms: 5.4, memory_mb: '90.0' },
    'mono-medium-standard-version-npm-check': {
      median_ms: 629.1,
      stddev_ms: 7.5,
      memory_mb: '94.4',
    },
    'mono-small-changesets-npm-check': { median_ms: 836.2, stddev_ms: 15.8, memory_mb: '104.9' },
    'mono-small-commit-and-tag-version-npm-check': {
      median_ms: 528.7,
      stddev_ms: 3.8,
      memory_mb: '89.4',
    },
    'mono-small-ferrflow-binary-check': { median_ms: 2.4, stddev_ms: 0.1, memory_mb: '12.5' },
    'mono-small-ferrflow-binary-release-dry-run': {
      median_ms: 18.4,
      stddev_ms: 0.2,
      memory_mb: '27.7',
    },
    'mono-small-ferrflow-binary-tag': { median_ms: 4.7, stddev_ms: 0.1, memory_mb: '12.6' },
    'mono-small-ferrflow-binary-validate': { median_ms: 2.4, stddev_ms: 0.1, memory_mb: '12.5' },
    'mono-small-ferrflow-binary-version': { median_ms: 2.4, stddev_ms: 0.1, memory_mb: '12.5' },
    'mono-small-ferrflow-docker-tag': { median_ms: 186.2, stddev_ms: 4.2, memory_mb: '45.5' },
    'mono-small-ferrflow-docker-validate': { median_ms: 185.6, stddev_ms: 4.0, memory_mb: '43.3' },
    'mono-small-ferrflow-docker-version': { median_ms: 186.9, stddev_ms: 11.0, memory_mb: '45.6' },
    'mono-small-ferrflow-npm-check': { median_ms: 299.7, stddev_ms: 3.3, memory_mb: '90.2' },
    'mono-small-ferrflow-npm-release-dry-run': {
      median_ms: 301.4,
      stddev_ms: 2.5,
      memory_mb: '89.2',
    },
    'mono-small-ferrflow-npm-tag': { median_ms: 298.6, stddev_ms: 6.0, memory_mb: '89.4' },
    'mono-small-ferrflow-npm-validate': { median_ms: 288.4, stddev_ms: 2.4, memory_mb: '89.1' },
    'mono-small-ferrflow-npm-version': { median_ms: 289.1, stddev_ms: 2.9, memory_mb: '89.7' },
    'mono-small-standard-version-npm-check': {
      median_ms: 548.2,
      stddev_ms: 6.5,
      memory_mb: '91.1',
    },
    'single-changesets-npm-check': { median_ms: 757.6, stddev_ms: 13.6, memory_mb: '103.3' },
    'single-commit-and-tag-version-npm-check': {
      median_ms: 532.9,
      stddev_ms: 13.7,
      memory_mb: '89.0',
    },
    'single-ferrflow-binary-check': { median_ms: 2.4, stddev_ms: 0.1, memory_mb: '12.5' },
    'single-ferrflow-binary-release-dry-run': {
      median_ms: 16.6,
      stddev_ms: 0.2,
      memory_mb: '21.6',
    },
    'single-ferrflow-binary-tag': { median_ms: 4.4, stddev_ms: 0.1, memory_mb: '12.6' },
    'single-ferrflow-binary-validate': { median_ms: 2.2, stddev_ms: 0.1, memory_mb: '12.4' },
    'single-ferrflow-binary-version': { median_ms: 2.3, stddev_ms: 0.1, memory_mb: '12.5' },
    'single-ferrflow-docker-tag': { median_ms: 186.2, stddev_ms: 4.4, memory_mb: '43.6' },
    'single-ferrflow-docker-validate': { median_ms: 183.9, stddev_ms: 3.4, memory_mb: '46.3' },
    'single-ferrflow-docker-version': { median_ms: 182.0, stddev_ms: 3.6, memory_mb: '44.1' },
    'single-ferrflow-npm-check': { median_ms: 299.9, stddev_ms: 2.9, memory_mb: '89.3' },
    'single-ferrflow-npm-release-dry-run': { median_ms: 303.2, stddev_ms: 4.3, memory_mb: '90.3' },
    'single-ferrflow-npm-tag': { median_ms: 299.9, stddev_ms: 5.9, memory_mb: '89.9' },
    'single-ferrflow-npm-validate': { median_ms: 290.3, stddev_ms: 2.7, memory_mb: '89.2' },
    'single-ferrflow-npm-version': { median_ms: 294.2, stddev_ms: 2.3, memory_mb: '89.2' },
    'single-standard-version-npm-check': { median_ms: 556.6, stddev_ms: 11.6, memory_mb: '91.1' },
  },
  install_sizes: {
    changesets: { npm: '26.0' },
    'commit-and-tag-version': { npm: '20.0' },
    ferrflow: { binary: '9.0', npm: '13.0' },
    'release-please': { npm: '95.0' },
    'semantic-release': { npm: '69.0' },
    'standard-version': { npm: '21.0' },
  },
};
