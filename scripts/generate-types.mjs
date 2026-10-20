import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { compile } from 'json-schema-to-typescript';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const schema = JSON.parse(readFileSync(join(root, 'schema/ferrflow.json'), 'utf8'));

for (const hook of Object.values(schema.$defs.hooks.properties)) {
  if (hook.type === 'string' && !hook.enum) {
    hook.tsType = 'string | HookFunction';
  }
}

const body = await compile(schema, 'FerrflowConfig', {
  bannerComment: '',
  additionalProperties: false,
  style: { singleQuote: true },
});

writeFileSync(
  join(root, 'npm/types/config.d.ts'),
  `import type { HookFunction } from './hook-context.js';\n\n${body}`,
);
