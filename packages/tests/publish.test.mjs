import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  parseArgs,
  publishEntries,
  publishPending,
} from '../scripts/lib/publish.mjs';
import { readPackages } from '../scripts/lib/workspace.mjs';

const DEFAULT_REGISTRY = 'https://registry.npmjs.org/';

test('parseArgs accepts separated option values', () => {
  assert.deepEqual(
    parseArgs(['--registry', 'https://example.test/', '--tag', 'next']),
    { dryRun: false, registry: 'https://example.test/', tag: 'next' },
  );
});

test('parseArgs accepts joined option values', () => {
  assert.deepEqual(parseArgs(['--registry=https://example.test/']), {
    dryRun: false,
    registry: 'https://example.test/',
    tag: null,
  });
  assert.deepEqual(parseArgs(['--tag=next']), {
    dryRun: false,
    registry: DEFAULT_REGISTRY,
    tag: 'next',
  });
});

test('parseArgs defaults to npmjs without a dist-tag', () => {
  const expected = { dryRun: false, registry: DEFAULT_REGISTRY, tag: null };
  assert.deepEqual(parseArgs([]), expected);
  assert.deepEqual(parseArgs(['--dry-run']), { ...expected, dryRun: true });
});

test('parseArgs rejects missing and unknown options', () => {
  assert.throws(
    () => parseArgs(['--registry']),
    /missing value for --registry/,
  );
  assert.throws(
    () => parseArgs(['--registry=']),
    /missing value for --registry/,
  );
  assert.throws(() => parseArgs(['--tag']), /missing value for --tag/);
  assert.throws(
    () => parseArgs(['--verbose', '1']),
    /unknown argument --verbose/,
  );
});

test('publishEntries plans platform packages before the wrapper', () => {
  const packages = readPackages();
  const entries = publishEntries(packages);
  assert.deepEqual(
    entries.map(entry => entry.name),
    [
      ...packages.platforms.map(platform => platform.entry.package),
      packages.wrapper.name,
    ],
  );
  assert.equal(entries.at(-1).dir, packages.wrapperDir);
});

test('publishPending keeps publishing after a package fails', () => {
  const entries = [
    { dir: '/platform-a', name: 'xidlc-linux-x64' },
    { dir: '/platform-b', name: 'xidlc-win32-arm64' },
    { dir: '/wrapper', name: 'xidlc' },
  ];
  const attempted = [];
  const report = publishPending({
    entries,
    isPublished: () => false,
    publish: dir => {
      attempted.push(dir);
      if (dir === '/platform-b') {
        throw new Error(`npm publish failed in ${dir}`);
      }
    },
    version: '1.2.3',
  });
  assert.deepEqual(attempted, ['/platform-a', '/platform-b', '/wrapper']);
  assert.deepEqual(report.published, ['xidlc-linux-x64', 'xidlc']);
  assert.deepEqual(report.skipped, []);
  assert.deepEqual(report.failures, [
    { name: 'xidlc-win32-arm64', reason: 'npm publish failed in /platform-b' },
  ]);
});

test('publishPending skips versions already on the registry', () => {
  const entries = [
    { dir: '/platform-a', name: 'xidlc-linux-x64' },
    { dir: '/wrapper', name: 'xidlc' },
  ];
  const attempted = [];
  const report = publishPending({
    entries,
    isPublished: name => name === 'xidlc-linux-x64',
    publish: dir => attempted.push(dir),
    version: '1.2.3',
  });
  assert.deepEqual(attempted, ['/wrapper']);
  assert.deepEqual(report.published, ['xidlc']);
  assert.deepEqual(report.skipped, ['xidlc-linux-x64']);
  assert.deepEqual(report.failures, []);
});

test('publishPending records registry query failures without publishing', () => {
  const entries = [{ dir: '/wrapper', name: 'xidlc' }];
  const report = publishPending({
    entries,
    isPublished: () => {
      throw new Error('cannot query xidlc@1.2.3');
    },
    publish: () => {
      throw new Error('must not publish an unverified version');
    },
    version: '1.2.3',
  });
  assert.deepEqual(report.published, []);
  assert.deepEqual(report.skipped, []);
  assert.deepEqual(report.failures, [
    { name: 'xidlc', reason: 'cannot query xidlc@1.2.3' },
  ]);
});
