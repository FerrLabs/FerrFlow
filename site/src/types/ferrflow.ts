export interface Commit {
  hash: string;
  message: string;
  author: string;
  timestamp: string;
}

export interface FerrflowCheckResponse {
  currentVersion: string;
  nextVersion: string;
  bumpType: 'major' | 'minor' | 'patch' | 'none';
  reason: string;
  affectedPackages?: string[];
}

export interface FerrflowConfig {
  workspace?: {
    remote: string;
    branch: string;
  };
  packages: Array<{
    name: string;
    path: string;
    changelog: string;
    versionedFiles: Array<{
      path: string;
      format: 'json' | 'toml' | 'xml';
    }>;
  }>;
}
