#!/usr/bin/env node

const { spawnSync } = require('node:child_process');
const { resolveBinaryPath } = require('../lib/resolve-binary');

const binaryPath = resolveBinaryPath();
const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
});

if (result.error) {
  throw result.error;
}

if (typeof result.status === 'number') {
  process.exit(result.status);
}

process.exit(1);
