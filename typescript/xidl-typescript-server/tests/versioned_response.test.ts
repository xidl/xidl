import assert from 'node:assert/strict';
import test from 'node:test';
import { createVersionedRouter } from '../src/index.ts';

test('version caching preserves wildcard and deduplicates header names on both response paths', async () => {
  const retiredRequest = new Request('http://localhost/items', {
    headers: { 'X-Version': 'old' },
  });
  for (const [vary, expected] of [
    ['*', '*'],
    ['Accept, x-version, X-VERSION', 'Accept, x-version'],
  ] as const) {
    const router = createVersionedRouter({
      onVersionError(error, request) {
        assert.equal(error.kind, 'retired');
        assert.equal(error.status, 410);
        assert.equal(error.rawValue, 'old');
        assert.equal(request, retiredRequest);
        return new Response('rejected', {
          headers: { Vary: vary },
          status: 418,
        });
      },
      parameter: { in: 'header', name: 'X-Version' },
      retired: ['old'],
      versions: [
        {
          handler: async () => new Response('ok', { headers: { Vary: vary } }),
          id: 'v1',
          value: '1',
        },
      ],
    });
    const accepted = await router(
      new Request(retiredRequest, {
        headers: { 'X-Version': '1' },
      }),
    );
    const rejected = await router(retiredRequest);
    assert.equal(accepted.headers.get('Vary'), expected);
    assert.equal(rejected.headers.get('Vary'), expected);
    assert.equal(rejected.status, 418);
    assert.equal(await rejected.text(), 'rejected');
  }
});

test('Fetch error responses preserve their identity', async () => {
  const networkError = Response.error();
  const route = createVersionedRouter({
    onVersionError: () => networkError,
    parameter: { in: 'header', name: 'X-Version' },
    versions: [{ handler: async () => networkError, id: 'v1', value: '1' }],
  });
  assert.equal(
    await route(new Request('http://localhost/items')),
    networkError,
  );
  const mounted = new Request('http://localhost/items', {
    headers: { 'X-Version': '1' },
  });
  assert.equal(await route(mounted), networkError);
});
