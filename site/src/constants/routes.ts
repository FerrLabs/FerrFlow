export const ROUTES = {
  // Public routes
  HOME: '/',
  DOCS: '/docs',
  BLOG: '/blog',
  CHANGELOG: '/changelog',
  
  // Auth routes
  LOGIN: '/login',
  SIGNUP: '/signup',
  FORGOT_PASSWORD: '/forgot-password',
  RESET_PASSWORD: '/reset-password',
  VERIFY_EMAIL: '/verify-email',
  
  // App routes (app.ferrflow.com)
  DASHBOARD: '/',
  ANALYTICS: '/analytics',
  TOKENS: '/tokens',
  PLAYGROUND: '/playground',
  SETTINGS: '/settings',
  SETTINGS_PROFILE: '/settings/profile',
  SETTINGS_SECURITY: '/settings/security',
  SETTINGS_NOTIFICATIONS: '/settings/notifications',
} as const;
