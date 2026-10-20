export interface HookCommit {
  hash: string;
  message: string;
  type?: string;
  scope?: string;
  breaking: boolean;
}

export interface HookFile {
  path: string;
  format: string;
}

export interface HookPackage {
  name: string;
  version: string;
  bump: string;
}

export interface HookContext {
  package: string;
  oldVersion: string;
  newVersion: string;
  bumpType: string;
  tag: string;
  packagePath: string;
  changelog: string;
  dryRun: boolean;
  isPrerelease: boolean;
  monorepo: boolean;
  channel: string | null;
  releaseUrl: string | null;
  commits: HookCommit[];
  bumpedFiles: HookFile[];
  allPackages: HookPackage[];
}

export type HookFunction = (ctx: HookContext) => void | Promise<void>;
