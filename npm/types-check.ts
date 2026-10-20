import { defineConfig } from './index.js';

defineConfig({ workspace: { hooks: { postTag: (ctx) => void ctx.newVersion } } });

// @ts-expect-error misspelled workspace key
defineConfig({ workspace: { hookz: {} } });

// @ts-expect-error unknown field on the hook context
defineConfig({ workspace: { hooks: { postTag: (ctx) => void ctx.nope } } });
