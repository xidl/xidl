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
  // A member added inside the element type leaves every older view with the
  // element count and keys intact: the sequence, the map and the fixed array
  // all drop the newest member per element instead of failing the container.
  const zoned: types.v3.Parcel = {
    ...head,
    address: { ...head.address, zone: 'dock-3' },
    by_tag: Object.fromEntries(
      Object.entries(head.by_tag).map(([tag, stop]) => [
        tag,
        { ...stop, zone: 'rear' },
      ]),
    ),
    fixed: head.fixed.map(stop => ({ ...stop, zone: 'bay-9' })),
    stops: head.stops.map(stop => ({ ...stop, zone: 'gate-2' })),
  };
  for (const version of ['v1', 'v2'] as const) {
    const view = take(a.downcastParcel(version, zoned, storedValues, hooks));
    assert.equal(Object.hasOwn(view.address, 'zone'), false, version);
    for (const stop of [...view.stops, ...view.fixed]) {
      assert.equal(Object.hasOwn(stop, 'zone'), false, version);
    }
    assert.equal(
      Object.values(view.by_tag).every(stop => !Object.hasOwn(stop, 'zone')),
      true,
      version,
    );
    assert.equal(view.stops.length, zoned.stops.length, version);
    assert.deepEqual(
      Object.keys(view.by_tag).sort(),
      Object.keys(zoned.by_tag).sort(),
      version,
    );
    assert.equal(view.fixed.length, zoned.fixed.length, version);
  }
  // A member retired inside every container element returns per element from
  // storage, an element without its row omits the member, and a business
  // refusal surfaces the full container path.
  const gatedStored: a.StoredValues = {
    ...storedValues,
    addressGate: source => (source.zip === '20002' ? 'B-12' : undefined),
  };
  const element = (zip: string): types.v3.Address => ({ ...head.address, zip });
  const gated: types.v3.Parcel = {
    ...head,
    by_tag: { dock: element('30003') },
    stops: [element('20002')],
  };
  for (const version of ['v1', 'v2'] as const) {
    const view = take(a.downcastParcel(version, gated, gatedStored, hooks));
    // The served releases both declare the retired member on every element.
    const stop = view.stops[0] as { gate?: string };
    const dock = view.by_tag.dock as { gate?: string };
    assert.equal(stop.gate, 'B-12', version);
    assert.equal(
      Object.hasOwn(dock, 'gate'),
      false,
      `${version}: storage without the element row omits the member`,
    );
  }
  const refusedStops = a.downcastParcel(
    'v2',
    { ...gated, stops: [element('20002'), element('90001')] },
    gatedStored,
    hooks,
  );
  assert.equal(refusedStops.ok, false);
  if (!refusedStops.ok) {
    assert.equal(
      refusedStops.error.typePath,
      'Parcel.stops.Address.gate',
      'the refusal names the sequence element’s member',
    );
  }
  const refusedMap = a.downcastParcel(
    'v2',
    { ...gated, by_tag: { dock: element('90001') } },
    gatedStored,
    hooks,
  );
  assert.equal(refusedMap.ok, false);
  if (!refusedMap.ok) {
    assert.equal(
      refusedMap.error.typePath,
      'Parcel.by_tag.Address.gate',
      'the refusal names the map element’s member',
    );
  }
  // An old client sending the retired member must see it acknowledged, not forwarded.
  const v2Head = take(
    a.decodeAndUpcastDepotShipFromV2(
      {
        parcel: { ...wireParcel, carrier: 'postal', handling_note: 'ignored' },
      },
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

  // The retired response header answers callers whose contract still declares
  // it, straight from storage; Head never carries it, and a trace with no
  // stored header cannot be served to 2026-04 at all.
  const trace = async () => ({
    ok: true as const,
    stored: storedValues,
    value: 'p1',
  });
  const tracedV2 = await a.dispatchDepotTraceFromV2({ id: 'p1' }, trace, hooks);
  assert(tracedV2.dispatched && tracedV2.projected && tracedV2.value.ok);
  assert.deepEqual(tracedV2.value.value, {
    return: 'p1',
    trace_id: 'trace-1',
  });
  const tracedHead = await a.dispatchDepotTraceFromV3(
    { id: 'p1' },
    trace,
    hooks,
  );
  assert(tracedHead.dispatched && tracedHead.projected && tracedHead.value.ok);
  // Head declared no retired header, so its outcome is the bare return value.
  assert.equal(tracedHead.value.value, 'p1');
  const untracedV2 = await a.dispatchDepotTraceFromV2(
    { id: 'never-traced' },
    async () => ({
      ok: true as const,
      stored: storedValues,
      value: 'never-traced',
    }),
    hooks,
  );
  assert(untracedV2.dispatched);
  assert.equal(untracedV2.projected, false);
  assert.equal(
    (untracedV2.head as { ok: true; value: string }).value,
    'never-traced',
    'the refusal retains the committed Head outcome',
  );
}
