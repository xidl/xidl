import assert from 'node:assert/strict';
import { v1 } from './v1/versioned_catalog.client.js';
import { Denied } from './v1/versioned_catalog.errors.js';
import { v2 } from './v2/versioned_catalog.client.js';
import { Denied as CurrentDenied } from './v2/versioned_catalog.errors.js';

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
const baseline = (await current.search([], 'shop-a', undefined)).return;
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
  const [oldResponse, headResponse] = await Promise.all([
    first.search([...filters], 'shop-a', undefined),
    current.search([...filters], 'shop-a', undefined),
  ]);
  assert.equal(oldResponse.bookmark, undefined);
  assert.equal(headResponse.bookmark, undefined);
  const old = oldResponse.return;
  const head = headResponse.return;
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
const unknown = (
  await first.search(['video', 'hologram', 'hologram'], 'shop-a', undefined)
).return;
assert.equal(unknown.products.length, 3);
assert.deepEqual(unknown.messages, [{ code: 'not_found', value: 'hologram' }]);
assert.deepEqual(
  (await current.search(['video'], 'shop-b', undefined)).return.products.map(
    product => product.id,
  ),
  ['private'],
);
await assert.rejects(first.search([], 'unknown', undefined), Denied);
for (const preference of [undefined, '', 'video']) {
  const [old, head] = await Promise.all([
    first.search(['video'], 'shop-a', preference),
    current.search(['video'], 'shop-a', preference),
  ]);
  assert.equal(old.bookmark, preference);
  assert.equal(head.bookmark, preference);
  assert.equal(old.return.products.length, 3);
  assert.equal(head.return.products.length, 3);
  const [oldPreference, headPreference] = await Promise.all([
    first.remember('shop-a', preference),
    current.remember('shop-a', preference),
  ]);
  assert.equal(oldPreference.bookmark, preference);
  assert.equal(headPreference.bookmark, preference);
}
await assert.rejects(current.remember('shop-a', 'hologram'), CurrentDenied);
assert.deepEqual(
  (await current.search([], 'shop-a', undefined)).return,
  baseline,
);
console.log(
  'PASS: independent TypeScript SDKs preserve Head catalog filtering',
);
