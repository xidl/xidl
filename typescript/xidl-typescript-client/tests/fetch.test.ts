import assert from 'node:assert/strict';
import test from 'node:test';

import { resolveFetch, resolveFetchLegacy } from '../src/index.js';

test('resolveFetch returns provided impl', () => {
  const impl = async () => new Response();
  assert.equal(resolveFetch(impl), impl);
});

test('resolveFetch resolves global fetch', () => {
  const fetched = resolveFetch();
  assert.equal(typeof fetched, 'function');
});

test('resolveFetchLegacy returns provided impl', () => {
  const impl = async () => new Response();
  assert.equal(resolveFetchLegacy(impl), impl);
});

test('resolveFetchLegacy resolves window.fetch when present', async () => {
  let called = false;
  const impl = async () => {
    called = true;
    return new Response();
  };
  (globalThis as { window?: { fetch: unknown } }).window = { fetch: impl };
  try {
    const fetched = resolveFetchLegacy();
    assert.equal(typeof fetched, 'function');
    await fetched('http://x');
    assert.equal(called, true);
  } finally {
    delete (globalThis as { window?: unknown }).window;
  }
});
