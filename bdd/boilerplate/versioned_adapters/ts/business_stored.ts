import assert from 'node:assert/strict';
import type * as types from './versioned_adapters_business';
import * as a from './versioned_adapters_business.adapters';

const take = <T>(
  result: { ok: true; value: T } | { ok: false; error: unknown },
): T => {
  if (!result.ok) throw new Error(JSON.stringify(result.error));
  return result.value;
};

/**
 * A member Head retired stays readable by every version that still declares
 * it. Its value comes from storage through typed readers; nothing is invented,
 * and a missing row is an explicit OMIT rather than a silent drop.
 */
export async function runStoredJourney(
  hooks: a.VersionAdapters,
  storedValues: a.StoredValues,
  head: types.v3.Parcel,
  wireParcel: Record<string, unknown>,
): Promise<void> {
  // A retired member reaches only the versions that still declare it, from storage.
  assert.deepEqual(a.NEEDS.Parcel, {
    '2026-01': ['handling_note', 'reference'],
    '2026-04': ['handling_note'],
    '2026-07': [],
  });
  // Operation responses are keyed by their version-map operation.
  assert.deepEqual(a.NEEDS['Depot.trace'], {
    '2026-01': ['trace_id'],
    '2026-04': ['trace_id'],
    '2026-07': [],
  });
  const v2Parcel = take(a.downcastParcel('v2', head, storedValues, hooks));
  assert.equal(v2Parcel.handling_note, 'fragile');
  assert.equal(
    take(a.downcastParcel('v1', head, storedValues, hooks)).handling_note,
    'fragile',
  );
  assert.equal(
    Object.hasOwn(
      take(a.downcastParcel('v3', head, storedValues, hooks)),
      'handling_note',
    ),
    false,
  );
  // Storage without the row leaves the member absent: OMIT, never a fabricated value.
  const unknownHead = { ...head, id: 'unknown' };
  assert.equal(
    Object.hasOwn(
      take(a.downcastParcel('v2', unknownHead, storedValues, hooks)),
      'handling_note',
    ),
    false,
  );
  // An old client sending the retired member must see it acknowledged, not forwarded.
  const v2Head = take(
    a.decodeAndUpcastDepotShipFromV2(
      { parcel: { ...wireParcel, carrier: 'postal', handling_note: 'ignored' } },
      hooks,
    ),
  ).parcel;
  assert.equal(Object.hasOwn(v2Head, 'handling_note'), false);
  assert.equal(
    a.decodeAndUpcastDepotShipFromV3(
      {
        parcel: {
          ...wireParcel,
          audit: 'x',
          carrier: 'postal',
          handling_note: 'future',
        },
      },
      hooks,
    ).ok,
    false,
    'Head rejects a member it no longer declares',
  );
  // Dispatch hands the handler's stored values to the whole downcast chain.
  const served = await a.dispatchDepotGetFromV1(
    { id: 'p1' },
    async () => ({ ok: true, stored: storedValues, value: head }),
    hooks,
  );
  assert(served.dispatched && served.projected && served.value.ok);
  assert.equal(served.value.value.legacy, 'TRACK-1');
  assert.equal(served.value.value.handling_note, 'fragile');
  const servedHead = await a.dispatchDepotGet(
    'v3',
    { id: 'p1' },
    async () => ({ ok: true, stored: storedValues, value: head }),
    hooks,
  );
  assert(servedHead.dispatched && servedHead.projected && servedHead.value.ok);
  assert.equal(Object.hasOwn(servedHead.value.value, 'handling_note'), false);
}
