import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

import {
  assertVersionsAligned,
  REPO_ROOT,
  readPackages,
} from '../scripts/lib/workspace.mjs';
import { PLATFORMS } from '../xidlc/src/platforms.js';

const PLATFORM_PACKAGES = Object.values(PLATFORMS).map(entry => entry.package);
const RELEASE_WORKFLOW = join(
  REPO_ROOT,
  '.github',
  'workflows',
  'publish-release.yml',
);

function readWorkflow() {
  return readFileSync(RELEASE_WORKFLOW, 'utf8');
}

function readNpmExtraFiles() {
  const path = join(REPO_ROOT, 'release-please-config.json');
  const config = JSON.parse(readFileSync(path, 'utf8'));
  return config.packages['.']['extra-files'].filter(entry =>
    entry.path.startsWith('packages/'),
  );
}

test('platform packages agree with the wrapper manifest', () => {
  const packages = readPackages();
  assert.equal(assertVersionsAligned(packages), packages.wrapper.version);
  assert.equal(packages.platforms.length, PLATFORM_PACKAGES.length);
});

test('optional dependencies cover every platform at the wrapper version', () => {
  const { wrapper } = readPackages();
  const pinned = Object.entries(wrapper.optionalDependencies ?? {});
  assert.deepEqual(
    pinned.map(([name]) => name).sort(),
    [...PLATFORM_PACKAGES].sort(),
  );
  for (const [name, range] of pinned) {
    assert.equal(range, wrapper.version, `${name} must pin ${wrapper.version}`);
  }
});

test('release-please bumps every npm version field', () => {
  const tracked = new Set(
    readNpmExtraFiles().map(entry => `${entry.path}#${entry.jsonpath}`),
  );
  const expected = [
    'packages/xidlc/package.json#$.version',
    ...PLATFORM_PACKAGES.map(
      name => `packages/xidlc/package.json#$.optionalDependencies.${name}`,
    ),
    ...PLATFORM_PACKAGES.map(name => `packages/${name}/package.json#$.version`),
  ];
  assert.deepEqual([...tracked].sort(), expected.sort());
});

test('the release workflow builds exactly the npm platform targets', () => {
  const targets = new Set(
    [...readWorkflow().matchAll(/^\s+- target: (\S+)$/gm)].map(
      match => match[1],
    ),
  );
  assert.deepEqual(
    [...targets].sort(),
    Object.values(PLATFORMS)
      .map(entry => entry.target)
      .sort(),
  );
});

test('the release workflow publishes through the npm scripts', () => {
  const workflow = readWorkflow();
  assert.match(workflow, /packages\/scripts\/fetch-binaries\.mjs/);
  assert.match(workflow, /packages\/scripts\/publish\.mjs/);
  assert.match(workflow, /id-token: write/);
});

test('git ignores the fetched binaries and copied licenses', () => {
  const ignore = readFileSync(join(REPO_ROOT, '.gitignore'), 'utf8');
  assert.match(ignore, /^\/packages\/xidlc-\*\/bin\/$/m);
  assert.match(ignore, /^\/packages\/\*\/LICENSE$/m);
});
