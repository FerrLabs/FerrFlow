export const TOKEN_SCOPES = {
  MCP_READ: 'mcp:read',
  MCP_WRITE: 'mcp:write',
  MCP_ADMIN: 'mcp:admin',
  ANALYTICS_READ: 'analytics:read',
  FERRFLOW_CHECK: 'ferrflow:check',
  FERRFLOW_RELEASE: 'ferrflow:release',
} as const;

export const SCOPE_DESCRIPTIONS: Record<string, string> = {
  'mcp:read': 'Read access via MCP (basic tools)',
  'mcp:write': 'Write access via MCP (advanced tools)',
  'mcp:admin': 'Full MCP administration',
  'analytics:read': 'Read analytics and metrics',
  'ferrflow:check': 'Use ferrflow check command',
  'ferrflow:release': 'Use ferrflow release command',
};

export type TokenScope = typeof TOKEN_SCOPES[keyof typeof TOKEN_SCOPES];
