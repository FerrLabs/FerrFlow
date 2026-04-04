export const TOKEN_SCOPES = {
  ANALYTICS_READ: 'analytics:read',
  FERRFLOW_CHECK: 'ferrflow:check',
  FERRFLOW_RELEASE: 'ferrflow:release',
} as const;

export const SCOPE_DESCRIPTIONS: Record<string, string> = {
  'analytics:read': 'Read analytics and metrics',
  'ferrflow:check': 'Use ferrflow check command',
  'ferrflow:release': 'Use ferrflow release command',
};

export type TokenScope = typeof TOKEN_SCOPES[keyof typeof TOKEN_SCOPES];
