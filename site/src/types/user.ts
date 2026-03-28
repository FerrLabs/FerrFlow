export interface User {
  id: string;
  email: string;
  username?: string;
  displayName?: string;
  avatarUrl?: string;
  emailVerified: boolean;
  createdAt: string;
  updatedAt: string;
  lastLoginAt?: string;
}

export interface OAuthProvider {
  provider: 'github' | 'gitlab' | 'google';
  email: string;
  username?: string;
  avatarUrl?: string;
  connectedAt: string;
}

export interface ApiToken {
  id: string;
  userId: string;
  tokenPrefix: string;
  name: string;
  scopes: string[];
  lastUsedAt?: string;
  lastUsedIp?: string;
  usageCount: number;
  expiresAt?: string;
  createdAt: string;
  revokedAt?: string;
}
