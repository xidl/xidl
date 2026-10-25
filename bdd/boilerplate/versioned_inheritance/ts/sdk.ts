import assert from 'node:assert/strict';
import { ZodError } from 'zod';
import { verifyImportedReaders } from './imports';
import { v1 } from './v1/versioned_inheritance.client';
import { Projection } from './v1/versioned_inheritance.errors';
import { v2 } from './v2/versioned_inheritance.client';
import { v3 } from './v3/versioned_inheritance.client';

const base = process.argv[2];
assert(base);
let fetches = 0;
const checkedFetch =
  (expected: string): typeof fetch =>
  async (input, init) => {
    fetches++;
    const request = new Request(input, init);
    assert.equal(request.headers.get('x-address-version'), expected);
    request.headers.set('authorization', 'Bearer test-acme');
    const response = await fetch(request);
    assert.equal(response.headers.get('x-address-version'), expected);
    return response;
  };
const first = new v1.shipping.AddressesClient(base, {
  fetch: checkedFetch('2026-01-01'),
});
const middle = new v2.shipping.AddressesClient(base, {
  fetch: checkedFetch('2026-04-01'),
});
const head = new v3.shipping.AddressesClient(base, {
  fetch: checkedFetch('2026-07-01'),
});
const written = await first.update('acme', 'a2', {
  coordinates: [0, -90],
  country_code: 'CA',
  delivery_note: '',
  delivery_reference: 'T-SDK',
  line: 'TS street',
  parent: { checkpoint: 'TS reroute', parent: { checkpoint: 'TS original' } },
  priority: 0,
});
assert.equal(written.id, 'a2');
assert.equal(written.delivery_reference, 'T-SDK');
assert.equal(written.delivery_note, '');
assert.equal(written.priority, 0);
assert.deepEqual(written.coordinates, [0, -90]);
const beforeInvalid = fetches;
for (const coordinates of [[], [1], [1, 2, 3]]) {
  await assert.rejects(
    first.update('acme', 'a2', {
      coordinates,
      delivery_reference: 'must-not-persist',
    }),
    ZodError,
  );
}
assert.equal(
  fetches,
  beforeInvalid,
  'wrong arity is refused before network I/O',
);
const parent = written.parent;
assert(parent);
assert.equal(parent.checkpoint, 'TS reroute');
assert(parent.parent);
assert.equal(parent.parent.checkpoint, 'TS original');
assert.equal(Object.hasOwn(parent.parent, 'parent'), false);
const latest = await head.update('acme', 'a2', {
  signature_required: false,
  street: 'TS Head',
});
assert.equal(Object.hasOwn(latest, 'delivery_reference'), false);
assert.deepEqual(latest.parent, written.parent);
assert.deepEqual(latest.coordinates, written.coordinates);
const view = await middle.read('acme', 'a2');
assert.equal(view.delivery_reference, 'T-SDK');
assert.equal(view.delivery_note, '');
await middle.update('acme', 'a2', {
  country_code_v2: view.country_code_v2,
  priority: view.priority,
  street: view.street,
});
assert.deepEqual(await head.read('acme', 'a2'), latest);
await assert.rejects(
  first.update('acme', 'a2', { country_code: 'Atlantis' }),
  Projection,
);
assert.deepEqual(await head.read('acme', 'a2'), latest);
await verifyImportedReaders(base);
console.log('PASS: independent TypeScript address SDKs');
