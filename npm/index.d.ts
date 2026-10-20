import type { FerrFlowConfiguration } from './types/config.js';

export type { HookCommit, HookContext, HookFile, HookFunction, HookPackage } from './types/hook-context.js';
export type { FerrFlowConfiguration as FerrflowConfig } from './types/config.js';
export type FerrflowConfigInput =
  | FerrFlowConfiguration
  | (() => FerrFlowConfiguration | Promise<FerrFlowConfiguration>);

export declare function defineConfig(config: FerrflowConfigInput): FerrflowConfigInput;
