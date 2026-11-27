import assert from 'node:assert/strict';
import test from 'node:test';
import { createVersionedRouter } from '../src/index.ts';

test('version routing preserves the original failure from the selected handler', async () => {
  const failure = new Error('storage unavailable');
  const router = createVersionedRouter({
    parameter: { in: 'header', name: 'X-Version' },
    versions: [
      {
        handler: async () => {
          throw failure;
        },
        id: 'v1',
        value: '2026-12-01',
      },
    ],
  });
  const request = new Request('http://localhost/items/42', {
    headers: { 'X-Version': '2026-12-01' },
  });
  await assert.rejects(router(request), error => error === failure);

  const recovered = await router(request).catch(error => {
    assert.equal(error, failure);
    return new Response('unavailable', {
      headers: { Vary: 'X-Version' },
      status: 503,
    });
  });
  assert.equal(recovered.status, 503);
  assert.equal(recovered.headers.get('Vary'), 'X-Version');
  assert.equal(await recovered.text(), 'unavailable');
});

test('an exception from the version rejection callback propagates as-is', async () => {
  const failure = new Error('version rejection callback failed');
  const router = createVersionedRouter({
    onVersionError(error) {
      assert.equal(error.kind, 'missing');
      assert.equal(error.rawValue, undefined);
      throw failure;
    },
    parameter: { in: 'header', name: 'X-Version' },
    versions: [
      {
        handler: async () => new Response('unreachable'),
        id: 'v1',
        value: '1',
      },
    ],
  });
  await assert.rejects(
    router(new Request('http://localhost/items/42')),
    error => error === failure,
  );
});
