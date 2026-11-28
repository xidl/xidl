import assert from 'node:assert/strict';
import {
  structurallyCompatible,
  updatePolicy,
} from './business_hook_contracts';
import type * as types from './versioned_adapters_business';
import * as a from './versioned_adapters_business.adapters';
import { v2, v3 } from './versioned_adapters_business.server';

const take = <T>(
  result: { ok: true; value: T } | { ok: false; error: unknown },
): T => {
  if (!result.ok) throw new Error(JSON.stringify(result.error));
  return result.value;
};

/**
 * The update round trip. A patch retiring a request member on Head must still
 * let older callers persist it through the handler, and everything the handler
 * persists must read back through the response chain unchanged.
 */
export async function runUpdateJourney(
  hooks: a.VersionAdapters,
  storedValues: a.StoredValues,
  handlingNotes: Map<string, string>,
  head: types.v3.Parcel,
): Promise<void> {
  // The handler owns the resource; dispatch passes it the very instance the
  // application supplied as `current`, so patches apply to one shared state.
  let current: types.v3.Parcel = { ...head };
  let writes = 0;
  const handler = async (
    request: { id: string; patch: { note?: string } },
    context: a.VersionContext,
    stored: a.DepotUpdateStored,
  ): Promise<
    a.OperationOutcome<types.v3.Parcel, a.FrameworkError> & {
      stored: a.StoredValues;
    }
  > => {
    writes++;
    const served = context.originalVersionId;
    assert.equal(typeof served, 'string');
    // The handler sees exactly the instance the application passed as current.
    assert.equal(served === 'v1' || served === 'v2' || served === 'v3', true);
    if (stored.handling_note !== undefined) {
      handlingNotes.set(request.id, stored.handling_note);
    }
    if (request.patch.note !== undefined) {
      current = { ...current, note: request.patch.note };
    }
    return { ok: true, stored: storedValues, value: current };
  };
  const refusePatch: a.VersionAdapters = {
    ...hooks,
    parcelPatchUpcastFromV2: () => ({
      error: {
        direction: 'upcast',
        reason: 'business decision refused',
        typePath: 'ParcelPatch',
        version: '2026-04',
      },
      ok: false,
    }),
  };

  // Write then read: the retired member a v2 caller patches persists, and the
  // downcast response reads the persisted value back through the same storage.
  const written = await a.dispatchDepotUpdate(
    'v2',
    { id: 'p1', patch: { handling_note: 'keep cold', note: 'fragile-two' } },
    current,
    handler,
    { ...hooks, parcelPatchUpcastFromV2: updatePolicy.parcelPatchUpcastFromV2 },
  );
  assert.equal(written.dispatched, true);
  assert.equal(written.projected, true);
  assert.equal(writes, 1);
  assert.equal(handlingNotes.get('p1'), 'keep cold');
  if (written.dispatched && written.projected && written.value.ok) {
    const served = written.value.value as types.v2.Parcel;
    assert.equal(served.handling_note, 'keep cold');
    assert.equal(served.note, 'fragile-two');
  }

  // Read then unchanged write: patching a parcel with exactly the values a v2
  // read produced must leave the projected view identical.
  const before = take(a.downcastParcel('v2', current, storedValues, hooks));
  const same = await a.dispatchDepotUpdate(
    'v2',
    {
      id: 'p1',
      patch: { handling_note: before.handling_note, note: before.note },
    },
    current,
    handler,
    { ...hooks, ...structurallyCompatible },
  );
  assert.equal(same.dispatched && same.projected && same.value.ok, true);
  if (same.dispatched && same.projected && same.value.ok) {
    assert.deepEqual(same.value.value as types.v2.Parcel, before);
  }
  assert.equal(writes, 2);

  // The Head arm bridges the bag: a v3 caller persists nothing, because no
  // step between its request and Head ever carries a retired member.
  const headDispatch = await a.dispatchDepotUpdate(
    'v3',
    { id: 'p1', patch: {} },
    current,
    async (request, _context, stored) => {
      assert.deepEqual(stored, {}, 'Head callers carry an empty bag');
      assert.equal(Object.hasOwn(stored, 'handling_note'), false);
      writes++;
      return {
        ok: true,
        stored: storedValues,
        value: { ...current, id: request.id },
      };
    },
    hooks,
  );
  assert.equal(headDispatch.dispatched && headDispatch.projected, true);
  assert.equal(writes, 3);

  // A refusal before the handler must not repeat or commit any write.
  const refused = await a.dispatchDepotUpdate(
    'v2',
    { id: 'p1', patch: { handling_note: 'lost', note: 'never' } },
    current,
    handler,
    refusePatch,
  );
  assert.equal(refused.dispatched, false);
  assert.equal(writes, 3);
  assert.equal(handlingNotes.get('p1'), 'keep cold');

  // Wire validation still rules the request envelope before any hook runs.
  for (const [version, raw, why] of [
    [
      'v3',
      { id: 'p1', patch: { handling_note: 'gone' } },
      'v3 no longer declares the member',
    ],
    [
      'v2',
      { id: 'p1', patch: { carrier: 'postal' } },
      'ParcelPatch never declared a carrier',
    ],
    ['v1', { patch: { note: 'x' } }, 'the id envelope is validated too'],
  ] as const) {
    const result = await a.dispatchDepotUpdate(
      version,
      raw,
      current,
      handler,
      hooks,
    );
    assert.equal(result.dispatched, false, `${version}: ${why}`);
  }
  assert.equal(writes, 3);

  // The typed decode API exposes the same bag for direct composition.
  const decoded = a.decodeAndUpcastDepotUpdateFromV1(
    { id: 'p1', patch: { handling_note: 'v1 keeps it too' } },
    current,
    hooks,
  );
  assert(decoded.ok);
  assert.deepEqual(decoded.stored, { handling_note: 'v1 keeps it too' });
  const plain = a.decodeAndUpcastDepotUpdateFromV1(
    { id: 'p1', patch: { note: 'no retired member' } },
    current,
    hooks,
  );
  assert(plain.ok);
  assert.equal(plain.stored.handling_note, undefined);
  let reads = 0;
  const unavailable = async () => assert.fail('Unexpected Depot operation');
  const service: Parameters<typeof a.bindVersionedInterfaces.Depot>[1] = {
    _get: unavailable,
    bindings: unavailable,
    blob: unavailable,
    choose: unavailable,
    contact: unavailable,
    extra: unavailable,
    head_only: unavailable,
    inspect: unavailable,
    optional_change: unavailable,
    receipt: unavailable,
    ship: unavailable,
    state: unavailable,
    trace: unavailable,
    update: handler,
  };
  const readers: Parameters<typeof a.bindVersionedInterfaces.Depot>[3] = {
    update: async (_version, request) => {
      const id: string = request.id;
      assert.equal(id, 'p1');
      assert.equal('extra' in request, false);
      reads++;
      return current;
    },
  };
  const bindUpdate = (
    source: Parameters<typeof a.bindVersionedInterfaces.Depot>[0],
  ) => {
    const bound = a.bindVersionedInterfaces.Depot(
      source,
      service,
      {
        ...hooks,
        ...structurallyCompatible,
      },
      readers,
    );
    const update = bound.operations.find(
      ({ operation }) => operation.handler === 'update',
    );
    assert(update);
    return update.binding;
  };
  const legacy = bindUpdate({ id: 'v2', tables: v2 });
  const invalid = await legacy.dispatch('v2', {
    id: 'p1',
    patch: { carrier: 'postal' },
  });
  assert.equal(invalid.dispatched, false);

  const unexpectedMember = await legacy.dispatch('v2', {
    extra: 'raw only',
    id: 'p1',
    patch: { note: 'new note' },
  });
  assert(
    !unexpectedMember.dispatched &&
      unexpectedMember.error.kind === 'Validation',
  );
  assert.equal(reads, 0);
  assert.equal(writes, 3);

  const updated = await legacy.dispatch('v2', {
    id: 'p1',
    patch: { note: 'new note' },
  });
  assert(updated.dispatched && updated.projected && updated.value.ok);
  assert.equal(reads, 1);
  assert.equal(writes, 4);

  const currentVersion = bindUpdate({ id: 'v3', tables: v3 });
  const updatedHead = await currentVersion.dispatch('v3', {
    id: 'p1',
    patch: {},
  });
  assert(
    updatedHead.dispatched && updatedHead.projected && updatedHead.value.ok,
  );
  assert.equal(reads, 1);
  assert.equal(writes, 5);

  console.log(
    'PASS: update round trip persists and reads back retired members',
  );
}
