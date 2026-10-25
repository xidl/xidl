import assert from 'node:assert/strict';
import { ZodError } from 'zod';
import { readers as head } from './imported/head_reader.client';
import { SavedAddressSchema } from './imported/head_reader.zod';
import { readers as old } from './imported/old_reader.client';
import { v1 } from './imported/versioned_inheritance.zod';

// An imported type fixes the consumer's contract. The application still owns
// HTTP version selection, so these readers explicitly set their request headers.
const pinnedFetch =
  (version: string): typeof fetch =>
  async (input, init) => {
    const request = new Request(input, init);
    request.headers.set('x-address-version', version);
    request.headers.set('authorization', 'Bearer test-acme');
    const response = await fetch(request);
    assert.equal(response.headers.get('x-address-version'), version);
    assert.equal(response.status, 200);
    return response;
  };

export async function verifyImportedReaders(base: string) {
  const before = await fetch(`${base}/audit`).then(response => response.json());
  const oldest = new old.AddressesClient(base, {
    fetch: pinnedFetch('2026-01-01'),
  });
  const latest = new head.AddressesClient(base, {
    fetch: pinnedFetch('2026-07-01'),
  });
  const first = await oldest.read('acme', 'a2');
  // Imported request DTOs pass through the same stored-bag write path. An empty
  // old patch preserves Head-only fields and adds no delivery history.
  assert.deepEqual(await oldest.update('acme', 'a2', {}), first);
  const current = await latest.read('acme', 'a2');
  assert.equal(first.id, 'a2');
  assert.equal(first.line, current.street);
  assert.deepEqual(first.parent, current.parent);
  assert.deepEqual(first.coordinates, current.coordinates);
  assert.equal(typeof first.delivery_reference, 'string');
  assert.equal(Object.hasOwn(current, 'delivery_reference'), false);
  assert.deepEqual(
    SavedAddressSchema.parse({ address: current }).address,
    current,
  );
  assert.equal(v1.shipping.AddressSchema.safeParse(current).success, false);

  // Returning Head bytes to a v1 consumer fails its generated response validator.
  const mismatched = new old.AddressesClient(base, {
    fetch: pinnedFetch('2026-07-01'),
  });
  await assert.rejects(() => mismatched.read('acme', 'a2'), ZodError);
  assert.deepEqual(
    await fetch(`${base}/audit`).then(response => response.json()),
    before,
  );
}
