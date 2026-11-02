import { copyFile, mkdir, readFile } from 'node:fs/promises';

// Read the locked package's declared browser build through its public root export.
const manifestUrl = new URL(
  '../package.json',
  import.meta.resolve('@scalar/api-reference'),
);
const manifest = JSON.parse(await readFile(manifestUrl, 'utf8'));
const output = new URL('../public/_scalar/', import.meta.url);
await mkdir(output, { recursive: true });
await copyFile(
  new URL(manifest.browser, manifestUrl),
  new URL('scalar.js', output),
);
await copyFile(
  new URL('../Scalar.LICENSE.txt', import.meta.url),
  new URL('LICENSE.txt', output),
);
