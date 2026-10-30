import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

import { REPO_ROOT, readPackages } from '../scripts/lib/workspace.mjs';

const HOOK_ID_LINE = /^- id: xidlc-fmt$/m;
const PINNED_DEPENDENCY =
  /^ {2}additional_dependencies: \[xidlc@(\d+\.\d+\.\d+)\] # x-release-please-version$/m;
const PINNED_REV = /^ {4}rev: v(\d+\.\d+\.\d+) # x-release-please-version$/m;

function readRepoFile(path) {
  return readFileSync(join(REPO_ROOT, path), 'utf8');
}

test('the xidlc pre-commit hook installs the npm package with node', () => {
  const hooks = readRepoFile('.pre-commit-hooks.yaml');
  assert.match(hooks, HOOK_ID_LINE);
  assert.match(hooks, /^ {2}language: node$/m);
  assert.doesNotMatch(hooks, /^ {2}language: rust$/m);
});

test('the xidlc pre-commit hook pins the published wrapper version', () => {
  const pinned = readRepoFile('.pre-commit-hooks.yaml').match(
    PINNED_DEPENDENCY,
  );
  assert.ok(pinned, 'hook must pin xidlc with a release-please marker');
  assert.equal(pinned[1], readPackages().wrapper.version);
});

test('the README pre-commit example revs the published wrapper version', () => {
  const rev = readRepoFile('README.md').match(PINNED_REV);
  assert.ok(rev, 'README hook example must carry a release-please marker');
  assert.equal(rev[1], readPackages().wrapper.version);
});

test('release-please bumps the pinned hook dependency and README rev', () => {
  const config = JSON.parse(readRepoFile('release-please-config.json'));
  const tracked = new Set(
    config.packages['.']['extra-files']
      .filter(entry => entry.type === 'generic')
      .map(entry => entry.path),
  );
  assert.ok(tracked.has('.pre-commit-hooks.yaml'));
  assert.ok(tracked.has('README.md'));
});
