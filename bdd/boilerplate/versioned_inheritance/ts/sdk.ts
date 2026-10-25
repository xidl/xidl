import assert from 'node:assert/strict';
import { verifyImportedReaders } from './imports';
import { v1 } from './v1/versioned_inheritance.client';
import { Projection } from './v1/versioned_inheritance.errors';
import { v2 } from './v2/versioned_inheritance.client';
import { v3 } from './v3/versioned_inheritance.client';

const base = process.argv[2];
assert(base);
const checkedFetch =
  (expected: string): typeof fetch =>
  async (input, init) => {
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
  country_code: 'CA',
  delivery_note: '',
  delivery_reference: 'T-SDK',
  line: 'TS street',
  priority: 0,
});
assert.equal(written.id, 'a2');
assert.equal(written.delivery_reference, 'T-SDK');
assert.equal(written.delivery_note, '');
assert.equal(written.priority, 0);
const latest = await head.update('acme', 'a2', {
  signature_required: false,
  street: 'TS Head',
});
assert.equal(Object.hasOwn(latest, 'delivery_reference'), false);
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
