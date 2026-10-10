#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';

import {
  assertVersionsAligned,
  binaryPath,
  ensureLicenses,
  readPackages,
} from './lib/workspace.mjs';

const DEFAULT_REGISTRY = 'https://registry.npmjs.org/';

function parseArgs(argv) {
  const options = { dryRun: false, registry: DEFAULT_REGISTRY, tag: null };
  for (let index = 0; index < argv.length; index += 1) {
    const name = argv[index];
    if (name === '--dry-run') {
      options.dryRun = true;
      continue;
    }
    const value = argv[index + 1];
    if (value === undefined) {
      throw new Error(`missing value for ${name}`);
    }
    switch (name) {
      case '--registry':
        options.registry = value;
        break;
      case '--tag':
        options.tag = value;
        break;
      default:
        throw new Error(`unknown argument ${name}`);
    }
    index += 1;
  }
  return options;
}

function assertReady(platform) {
  const target = binaryPath(platform);
  if (!existsSync(target)) {
    throw new Error(
      `missing prebuilt binary ${target}; run: just npm-fetch-binaries`,
    );
  }
  if (statSync(target).size === 0) {
    throw new Error(`empty prebuilt binary ${target}; refetch it`);
  }
}

/** Tell published versions apart from unpublished ones; other failures are fatal. */
function isPublished(name, version, registry) {
  const result = spawnSync(
    'npm',
    ['view', `${name}@${version}`, 'version', `--registry=${registry}`],
    { encoding: 'utf8' },
  );
  if (result.status === 0) {
    return true;
  }
  const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
  if (output.includes('E404')) {
    return false;
  }
  throw new Error(
    `cannot query ${name}@${version} on ${registry}:\n${output.trim()}`,
  );
}

function publish(dir, options) {
  const args = ['publish', `--registry=${options.registry}`];
  if (options.dryRun) {
    args.push('--dry-run');
  }
  if (options.tag !== null) {
    args.push(`--tag=${options.tag}`);
  }
  const result = spawnSync('npm', args, { cwd: dir, stdio: 'inherit' });
  if (result.status !== 0) {
    throw new Error(`npm publish failed in ${dir}`);
  }
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const packages = readPackages();
  const version = assertVersionsAligned(packages);
  for (const platform of packages.platforms) {
    assertReady(platform);
  }
  ensureLicenses(packages);
  const pending = [
    ...packages.platforms.map(platform => ({
      dir: platform.dir,
      name: platform.entry.package,
    })),
    { dir: packages.wrapperDir, name: packages.wrapper.name },
  ];
  let skipped = 0;
  for (const entry of pending) {
    if (isPublished(entry.name, version, options.registry)) {
      console.log(`${entry.name}@${version} is already published; skipping`);
      skipped += 1;
      continue;
    }
    publish(entry.dir, options);
  }
  console.log(
    `${options.dryRun ? 'dry run: ' : ''}${pending.length - skipped} published, ` +
      `${skipped} skipped, version ${version}`,
  );
}

try {
  main();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
