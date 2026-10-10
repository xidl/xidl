import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';

import { PLATFORMS } from './platforms.js';

const require = createRequire(import.meta.url);

/** Supported `${platform}-${arch}` keys, in a stable order. */
export const SUPPORTED_PLATFORMS = Object.keys(PLATFORMS);

/**
 * Resolve the prebuilt binary for one platform.
 *
 * `resolve` defaults to module resolution from this package and is injectable
 * so tests can resolve without an installed platform package.
 */
export function resolveBinary(
  platform,
  arch,
  resolve = id => require.resolve(id),
) {
  const key = `${platform}-${arch}`;
  const entry = PLATFORMS[key];
  if (entry === undefined) {
    throw new Error(
      `xidlc: no prebuilt binary for ${key}; ` +
        `supported platforms: ${SUPPORTED_PLATFORMS.join(', ')}`,
    );
  }
  try {
    return resolve(`${entry.package}/bin/${entry.binary}`);
  } catch (cause) {
    throw new Error(
      `xidlc: optional dependency ${entry.package} is missing; ` +
        `install it with: npm install -g ${entry.package}`,
      { cause },
    );
  }
}

/**
 * Run a binary with inherited stdio and return its exit code.
 *
 * `spawn` defaults to `spawnSync` and is injectable for tests.
 */
export function runBinary(binary, args, spawn = spawnSync) {
  const result = spawn(binary, args, { stdio: 'inherit' });
  if (result.error !== undefined && result.error !== null) {
    throw new Error(`xidlc: cannot execute ${binary}`, { cause: result.error });
  }
  if (result.signal !== undefined && result.signal !== null) {
    process.kill(process.pid, result.signal);
  }
  return result.status ?? 1;
}

/**
 * Run the `xidlc` CLI and return the process exit code.
 *
 * `XIDLC_BINARY_PATH` overrides binary resolution, which is how a locally
 * built compiler can be exercised through the npm entry point.
 */
export function main(options = {}) {
  const {
    args = process.argv.slice(2),
    arch = process.arch,
    env = process.env,
    log = console,
    platform = process.platform,
    resolve,
    spawn,
  } = options;
  try {
    const binary =
      env.XIDLC_BINARY_PATH ?? resolveBinary(platform, arch, resolve);
    return runBinary(binary, args, spawn);
  } catch (error) {
    log.error(error.message);
    return 1;
  }
}
