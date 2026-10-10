import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

import { binaryPath, readPackages } from '../scripts/lib/workspace.mjs';
import { main, resolveBinary } from '../xidlc/src/launcher.js';
import { PLATFORMS } from '../xidlc/src/platforms.js';

const SHIM = fileURLToPath(new URL('../xidlc/bin/xidlc.js', import.meta.url));
const EXIT_CODE = fileURLToPath(
  new URL('./fixtures/exit-code.mjs', import.meta.url),
);
const TERMINATE = fileURLToPath(
  new URL('./fixtures/terminate.mjs', import.meta.url),
);
const WINDOWS = process.platform === 'win32';
const NATIVE = nativeBinary();

function nativeBinary() {
  const platform = readPackages().platforms.find(
    entry => entry.platformKey === `${process.platform}-${process.arch}`,
  );
  if (platform === undefined) {
    return null;
  }
  const target = binaryPath(platform);
  return existsSync(target) ? target : null;
}

test('resolveBinary requests the binary of each platform package', () => {
  for (const [key, entry] of Object.entries(PLATFORMS)) {
    const [platform, arch] = key.split('-');
    const requested = [];
    const resolved = resolveBinary(platform, arch, id => {
      requested.push(id);
      return `/fake/${entry.package}`;
    });
    assert.equal(resolved, `/fake/${entry.package}`);
    assert.deepEqual(requested, [`${entry.package}/bin/${entry.binary}`]);
  }
});

test('resolveBinary rejects unknown platforms by name', () => {
  assert.throws(
    () => resolveBinary('freebsd', 'x64'),
    /xidlc: no prebuilt binary for freebsd-x64/,
  );
  assert.throws(
    () => resolveBinary('linux', 'ia32'),
    /supported platforms: darwin-arm64/,
  );
});

test('resolveBinary names the platform package to install', () => {
  assert.throws(
    () =>
      resolveBinary('linux', 'x64', () => {
        throw new Error('MODULE_NOT_FOUND');
      }),
    /npm install -g xidlc-linux-x64/,
  );
});

test('main returns the exit code of the binary', () => {
  const calls = [];
  const code = main({
    arch: 'x64',
    args: ['fmt', '--inplace'],
    platform: 'linux',
    resolve: () => '/fake/xidlc',
    spawn: (binary, args, options) => {
      calls.push({ args, binary, options });
      return { signal: null, status: 7 };
    },
  });
  assert.equal(code, 7);
  assert.deepEqual(calls, [
    {
      args: ['fmt', '--inplace'],
      binary: '/fake/xidlc',
      options: { stdio: 'inherit' },
    },
  ]);
});

test('main reports an unsupported platform instead of throwing', () => {
  const errors = [];
  const code = main({
    arch: 'sparc',
    log: { error: message => errors.push(message) },
    platform: 'sunos',
  });
  assert.equal(code, 1);
  assert.match(errors[0], /sunos-sparc/);
});

test('main reports a spawn failure', () => {
  const errors = [];
  const code = main({
    arch: 'x64',
    env: {},
    log: { error: message => errors.push(message) },
    platform: 'linux',
    resolve: () => '/fake/xidlc',
    spawn: () => ({ error: new Error('EACCES'), signal: null, status: null }),
  });
  assert.equal(code, 1);
  assert.match(errors[0], /cannot execute \/fake\/xidlc/);
});

test('main falls back to 1 when the binary reports no status', () => {
  const code = main({
    arch: 'x64',
    env: {},
    platform: 'linux',
    resolve: () => '/fake/xidlc',
    spawn: () => ({ signal: null, status: null }),
  });
  assert.equal(code, 1);
});

test('main honors XIDLC_BINARY_PATH', () => {
  let seen = null;
  const code = main({
    arch: 'x64',
    env: { XIDLC_BINARY_PATH: '/opt/xidlc' },
    platform: 'linux',
    resolve: () => {
      throw new Error('resolution must not run');
    },
    spawn: binary => {
      seen = binary;
      return { signal: null, status: 0 };
    },
  });
  assert.equal(code, 0);
  assert.equal(seen, '/opt/xidlc');
});

test('the shim forwards arguments and exit codes', { skip: WINDOWS }, () => {
  const result = spawnSync(process.execPath, [SHIM, '--version'], {
    encoding: 'utf8',
    env: { ...process.env, XIDLC_BINARY_PATH: EXIT_CODE },
  });
  assert.equal(result.status, 7);
  assert.equal(result.stdout.trim(), 'args: --version');
});

test('the shim propagates the signal of the binary', { skip: WINDOWS }, () => {
  const result = spawnSync(process.execPath, [SHIM], {
    env: { ...process.env, XIDLC_BINARY_PATH: TERMINATE },
  });
  assert.equal(result.signal, 'SIGTERM');
});

test('the fetched native binary runs', {
  skip: WINDOWS || NATIVE === null,
}, () => {
  const result = spawnSync(NATIVE, ['--version'], { encoding: 'utf8' });
  assert.equal(result.status, 0);
  assert.match(result.stdout, /^xidlc \d+\.\d+\.\d+/);
});
