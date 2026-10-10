#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';

import { parseArgs, publishEntries, publishPending } from './lib/publish.mjs';
import {
  assertVersionsAligned,
  binaryPath,
  ensureLicenses,
  readPackages,
} from './lib/workspace.mjs';

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

/** Tell published versions apart from unpublished ones; other registry errors propagate. */
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
  const report = publishPending({
    entries: publishEntries(packages),
    isPublished: name => isPublished(name, version, options.registry),
    publish: dir => publish(dir, options),
    version,
  });
  for (const name of report.skipped) {
    console.log(`${name}@${version} is already published; skipping`);
  }
  for (const failure of report.failures) {
    console.error(`cannot publish ${failure.name}: ${failure.reason}`);
  }
  console.log(
    `${options.dryRun ? 'dry run: ' : ''}${report.published.length} published, ` +
      `${report.skipped.length} skipped, ${report.failures.length} failed, ` +
      `version ${version}`,
  );
  if (report.failures.length > 0) {
    process.exitCode = 1;
  }
}

try {
  main();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
