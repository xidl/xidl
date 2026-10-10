import { copyFileSync, existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { PLATFORMS } from '../../xidlc/src/platforms.js';

/** Absolute path of the `packages/` directory holding every published package. */
export const NPM_ROOT = fileURLToPath(new URL('../..', import.meta.url));

/** Absolute path of the repository root. */
export const REPO_ROOT = fileURLToPath(new URL('../../..', import.meta.url));

function readManifest(dir) {
  const manifestPath = join(dir, 'package.json');
  let text;
  try {
    text = readFileSync(manifestPath, 'utf8');
  } catch (cause) {
    throw new Error(`cannot read npm manifest ${manifestPath}`, { cause });
  }
  return JSON.parse(text);
}

/**
 * Read the wrapper package and every platform package manifest.
 *
 * Each platform entry carries its `PLATFORMS` record, `platformKey`, `dir`
 * and `manifest`.
 */
export function readPackages() {
  return {
    platforms: Object.entries(PLATFORMS).map(([platformKey, entry]) => {
      const dir = join(NPM_ROOT, entry.package);
      return { dir, entry, manifest: readManifest(dir), platformKey };
    }),
    wrapper: readManifest(join(NPM_ROOT, 'xidlc')),
    wrapperDir: join(NPM_ROOT, 'xidlc'),
  };
}

/** Absolute path of the prebuilt binary inside a platform package. */
export function binaryPath(platform) {
  return join(platform.dir, 'bin', platform.entry.binary);
}

/**
 * Throw when wrapper and platform metadata disagree.
 *
 * Returns the shared version so callers can reuse it.
 */
export function assertVersionsAligned(packages) {
  const { version } = packages.wrapper;
  const optionalDependencies = packages.wrapper.optionalDependencies ?? {};
  const expected = new Set(
    packages.platforms.map(platform => platform.entry.package),
  );
  const problems = [];
  for (const platform of packages.platforms) {
    if (platform.manifest.name !== platform.entry.package) {
      problems.push(
        `${platform.dir}: package name is ${platform.manifest.name}, ` +
          `expected ${platform.entry.package}`,
      );
    }
    if (platform.manifest.version !== version) {
      problems.push(
        `${platform.manifest.name}: version is ${platform.manifest.version}, ` +
          `expected ${version}`,
      );
    }
    const pinned = optionalDependencies[platform.entry.package];
    if (pinned !== version) {
      problems.push(
        `xidlc optionalDependencies.${platform.entry.package} is ${pinned}, ` +
          `expected ${version}`,
      );
    }
  }
  for (const name of Object.keys(optionalDependencies)) {
    if (!expected.has(name)) {
      problems.push(`xidlc declares unknown optional dependency ${name}`);
    }
  }
  if (problems.length > 0) {
    throw new Error(
      `npm package metadata is inconsistent:\n- ${problems.join('\n- ')}`,
    );
  }
  return version;
}

/**
 * Copy the repository LICENSE into every package that will be published.
 *
 * npm always ships a package's LICENSE file, so this copy is what carries the
 * Apache-2.0 text into the published artifacts.
 */
export function ensureLicenses(packages) {
  const license = join(REPO_ROOT, 'LICENSE');
  if (!existsSync(license)) {
    throw new Error(`cannot find the repository license at ${license}`);
  }
  const dirs = [
    packages.wrapperDir,
    ...packages.platforms.map(entry => entry.dir),
  ];
  return dirs.map(dir => {
    const target = join(dir, 'LICENSE');
    if (!existsSync(target)) {
      copyFileSync(license, target);
    }
    return target;
  });
}
