import { cpSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const websiteDir = join(dirname(fileURLToPath(import.meta.url)), '..');

cpSync(
  join(websiteDir, '..', 'CHANGELOG.md'),
  join(websiteDir, 'public', 'changelog.md'),
);
