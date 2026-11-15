export type ChangelogProduct =
  'ferrflow' | 'ferrvault' | 'ferrtrack' | 'ferrgrowth' | 'ferrfleet' | 'ferrlens' | 'ferrlabs';

export type ChangelogType = 'new' | 'fix' | 'perf' | 'breaking' | 'deprecation' | 'security';

export interface ChangelogAttributes {
  title: string;
  summary: string;
  date: string;
  product: ChangelogProduct;
  type: ChangelogType;
  prLink?: string;
  docsLink?: string;
  draft?: boolean;
}

export interface ProductMeta {
  readonly name: string;
  readonly accent: string;
  readonly url: string;
}

export const PRODUCT_META: Record<ChangelogProduct, ProductMeta> = {
  ferrflow: { name: 'FerrFlow', accent: '#e8733a', url: 'https://ferrflow.com' },
  ferrvault: { name: 'FerrVault', accent: '#10b981', url: 'https://ferrvault.com' },
  ferrtrack: { name: 'FerrTrack', accent: '#6366f1', url: 'https://ferrtrack.com' },
  ferrgrowth: { name: 'FerrGrowth', accent: '#7c3aed', url: 'https://ferrgrowth.com' },
  ferrfleet: { name: 'FerrFleet', accent: '#f59e0b', url: 'https://ferrfleet.com' },
  ferrlens: { name: 'FerrLens', accent: '#14b8a6', url: 'https://ferrlens.com' },
  ferrlabs: { name: 'FerrLabs', accent: '#1e293b', url: 'https://ferrlabs.com' },
};

export const TYPE_LABEL_SHORT: Record<ChangelogType, string> = {
  new: 'NEW',
  fix: 'FIX',
  perf: 'PERF',
  breaking: 'BREAKING',
  deprecation: 'DEPRECATION',
  security: 'SECURITY',
};

export const TYPE_LABEL_LONG: Record<ChangelogType, string> = {
  new: 'NEW',
  fix: 'FIX',
  perf: 'PERFORMANCE',
  breaking: 'BREAKING',
  deprecation: 'DEPRECATION',
  security: 'SECURITY',
};

export const CHANGELOG_PRODUCT: ChangelogProduct = 'ferrflow';
