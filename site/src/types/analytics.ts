export interface AnalyticsOverview {
  totalEvents: number;
  totalReleases: number;
  totalPackages: number;
  totalUsers: number;
  bumps: {
    major: number;
    minor: number;
    patch: number;
  };
  topPackages: Array<{
    name: string;
    releases: number;
  }>;
}

export interface Release {
  id: string;
  packageName: string;
  versionFrom: string;
  versionTo: string;
  bumpType: 'major' | 'minor' | 'patch';
  timestamp: string;
  executionTimeMs?: number;
  success: boolean;
}

export interface PublicStats {
  totalReleases: number;
  totalPackages: number;
  totalUsers: number;
  majorBumps: number;
  minorBumps: number;
  patchBumps: number;
}
