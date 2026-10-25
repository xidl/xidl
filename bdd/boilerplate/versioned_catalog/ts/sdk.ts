import assert from 'node:assert/strict';
import { v1 } from './v1/versioned_catalog.client.js';
import { Denied } from './v1/versioned_catalog.errors.js';
import { v2 } from './v2/versioned_catalog.client.js';

const base = process.argv[2];
assert(base);
const checkedFetch =
  (version: string): typeof fetch =>
  async (input, init) => {
    const request = new Request(input, {
      ...init,
      signal: AbortSignal.timeout(5000),
    });
    assert.equal(request.headers.get('X-Catalog-Version'), version);
    const response = await fetch(request);
    assert.equal(response.headers.get('X-Catalog-Version'), version);
    return response;
  };
const first = new v1.CatalogClient(base, { fetch: checkedFetch('2026-07-01') });
const current = new v2.CatalogClient(base, {
  fetch: checkedFetch('2026-10-01'),
});
const baseline = await current.search([], 'shop-a');
assert.equal(baseline.products.length, 4);
assert.equal(baseline.products[3]?.assets[0]?.kind, 'video');
for (const [filters, ids] of [
  [[], ['image', 'clip', 'model', 'video-only']],
  [['video'], ['clip', 'model', 'video-only']],
  [['video', 'model_3d'], ['model']],
  [
    ['video', 'video'],
    ['clip', 'model', 'video-only'],
  ],
] as const) {
  const [old, head] = await Promise.all([
    first.search([...filters], 'shop-a'),
    current.search([...filters], 'shop-a'),
  ]);
  assert.deepEqual(
    old.products.map(product => product.id),
    ids,
  );
  assert.deepEqual(
    head.products.map(product => product.id),
    ids,
  );
  assert.deepEqual(old.messages, []);
  assert.deepEqual(head.messages, []);
  for (const product of old.products) {
    for (const image of product.images)
      assert.deepEqual(Object.keys(image), ['url']);
    if (product.id === 'video-only') assert.deepEqual(product.images, []);
  }
}
const unknown = await first.search(['video', 'hologram', 'hologram'], 'shop-a');
assert.equal(unknown.products.length, 3);
assert.deepEqual(unknown.messages, [{ code: 'not_found', value: 'hologram' }]);
assert.deepEqual(
  (await current.search(['video'], 'shop-b')).products.map(
    product => product.id,
  ),
  ['private'],
);
await assert.rejects(first.search([], 'unknown'), Denied);
assert.deepEqual(await current.search([], 'shop-a'), baseline);
console.log(
  'PASS: independent TypeScript SDKs preserve Head catalog filtering',
);
