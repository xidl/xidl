import assert from 'node:assert/strict';
import { runHttpJourney } from './business_http';
import { runStoredJourney } from './business_stored';
import { runUpdateJourney } from './business_update';
import * as a from './versioned_adapters_business.adapters';
import * as errors from './versioned_adapters_business.errors';

const ok = <T>(value: T): a.VersionedResult<T> => ({ ok: true, value });
const take = <T>(
  result: { ok: true; value: T } | { ok: false; error: unknown },
): T => {
  if (!result.ok) throw new Error(JSON.stringify(result.error));
  return result.value;
};
const refuse = (
  version: string,
  typePath: string,
): a.VersionedResult<never> => ({
  error: {
    direction: 'downcast',
    reason: 'outcome cannot be represented',
    typePath,
    version,
  },
  ok: false,
});
let hookCalls = 0;
let exceptionHooks = 0;
// Storage keeps what Head no longer carries while older versions still read it.
// Readers resolve identity from the Head instance; the chain never touches storage.
const storage = {
  handlingNotes: new Map<string, string>([['p1', 'fragile']]),
  optionalNotes: new Map<string, string>([['kept', 'kept']]),
  traces: new Map<string, string>([['p1', 'trace-1']]),
};
const storedValues: a.StoredValues = {
  // The trace header left the Head response; storage keeps it per traced id.
  depotTraceResponseTraceId: ({ return: id }) => storage.traces.get(id),
  optionalChangeNote: source => storage.optionalNotes.get(source.note ?? ''),
  parcelHandlingNote: source => storage.handlingNotes.get(source.id),
  // Never stored: the hook derives the retired reference from the tracking number.
  parcelLegacy: () => undefined,
};
const hooks: a.VersionAdapters = {
  choiceDowncastToV1: source => ok({ text: String(source.count) }),
  depotGetBusyErrorDowncastToV1: source => {
    exceptionHooks++;
    return ok(new errors.v1$NotFound({ id: source.reason }));
  },
  depotGetDeniedErrorDowncastToV2: source => {
    exceptionHooks++;
    return ok(new errors.v2$Busy({ reason: source.reason }));
  },
  depotGetFrameworkErrorDowncastToV2: source => {
    exceptionHooks++;
    return ok(new errors.v2$Busy({ reason: source.reason }));
  },
  depotTraceResponseDowncastToV2: (source, stored) => {
    const traceId = stored.depotTraceResponseTraceId(source);
    return traceId === undefined
      ? refuse('2026-04', 'Depot.trace')
      : ok({ trace_id: traceId });
  },
  depotUpdateBusyErrorDowncastToV1: source => {
    exceptionHooks++;
    return ok(new errors.v1$NotFound({ id: source.reason }));
  },
  depotUpdateDeniedErrorDowncastToV2: source => {
    exceptionHooks++;
    return ok(new errors.v2$Busy({ reason: source.reason }));
  },
  depotUpdateFrameworkErrorDowncastToV2: source => {
    exceptionHooks++;
    return ok(new errors.v2$Busy({ reason: source.reason }));
  },
  optionalChangeDowncastToV1: (source, stored) =>
    ok({ note: stored.optionalChangeNote(source) ?? a.OMIT }),
  optionalChangeUpcastFromV1: source =>
    ok({ ignored$note: a.IGNORED, note: source.note }),
  parcelDowncastToV1: source => ok({ legacy: source.tracking }),
  parcelDowncastToV2: (source, stored) =>
    ok({ handling_note: stored.parcelHandlingNote(source) ?? a.OMIT }),
  parcelPatchUpcastFromV2: source =>
    ok({
      handling_note: a.IGNORED,
      stored: { handling_note: source.handling_note },
    }),
  parcelUpcastFromV1: source => {
    hookCalls++;
    return ok({
      carrier: 'postal',
      courier_note: undefined,
      legacy: a.IGNORED,
      tracking: source.legacy,
    });
  },
  parcelUpcastFromV2: () =>
    ok({
      audit: 'checked',
      handling_note: a.IGNORED,
      // v2 callers never asked for a signature: the implied value, not the default.
      signature_required: false,
    }),
  stateDowncastToV1: () => refuse('2026-01', 'State'),
  stateDowncastToV2: () => ok('shipping' as const),
};
const wireParcel = {
  address: { zip_code: '10001' },
  by_tag: { home: { zip_code: '30003' } },
  fixed: [{ zip_code: '40004' }, { zip_code: '50005' }],
  id: 'p1',
  reference: 'TRACK-1',
  stops: [{ zip_code: '20002' }],
};
const head = take(
  a.decodeAndUpcastDepotShipFromV1({ parcel: wireParcel }, hooks),
).parcel;
assert.equal(head.tracking, 'TRACK-1');
assert.equal(head.carrier, 'postal');
assert.equal(head.address.zip, '10001');
assert.equal(head.by_tag.home.zip, '30003');
assert.equal(head.fixed[1].zip, '50005');
assert.equal(Object.hasOwn(head, 'note'), false);
assert.equal(Object.hasOwn(head, 'inspected'), false);
assert.equal(Object.hasOwn(head.address, 'zone'), false);
assert.equal(Object.hasOwn(head, 'reference'), false);
const resolvedVersion: 'v1' | 'v2' = process.env.TEST_V2 ? 'v2' : 'v1';
const projected = take(
  a.downcastParcel(resolvedVersion, head, storedValues, hooks),
);
assert.equal(projected.id, 'p1');

// Each invalid wire value must stop before any hook or business side effect.
const invalid: unknown[] = [
  { ...wireParcel, carrier: 'future' },
  { ...wireParcel, address: { zip_code: '10001', zone: 'future' } },
  { ...wireParcel, stops: [{ zip_code: 'x', zone: 'future' }] },
  { ...wireParcel, by_tag: { home: { zip_code: 'x', zone: 'future' } } },
  { ...wireParcel, fixed: [{ zip_code: 'x' }] },
  {
    ...wireParcel,
    fixed: [{ zip_code: 'x' }, { zip_code: 'y', zone: 'future' }],
  },
  { ...wireParcel, address: {} },
  { ...wireParcel, address: { zip: 'model-name-is-not-wire-name' } },
  { ...wireParcel, reference: undefined },
  null,
];
const callsBeforeInvalid = hookCalls;
for (const parcel of invalid) {
  const result = a.decodeAndUpcastDepotShipFromV1({ parcel }, hooks);
  assert.equal(result.ok, false, JSON.stringify(parcel));
  if (!result.ok)
    assert.equal('kind' in result.error && result.error.kind, 'Validation');
}
assert.equal(hookCalls, callsBeforeInvalid);
assert.equal(
  a.decodeAndUpcastDepotStateFromV1({ state: 'delivered' }, hooks).ok,
  false,
);
assert.equal(
  a.decodeAndUpcastDepotStateFromV1({ state: 'invalid' }, hooks).ok,
  false,
);
assert.deepEqual(
  take(a.decodeAndUpcastDepotStateFromV1({ state: 'pending' }, hooks)),
  { state: 'pending' },
);
assert.equal(
  a.decodeAndUpcastDepotChooseFromV1({ choice: { count: 1 } }, hooks).ok,
  false,
);
assert.equal(
  a.decodeAndUpcastDepotChooseFromV2({ choice: { count: 1, text: 'x' } }, hooks)
    .ok,
  false,
);
assert.deepEqual(
  take(a.decodeAndUpcastDepotChooseFromV2({ choice: { count: 1 } }, hooks)),
  { choice: { count: 1 } },
);
assert.deepEqual(
  take(
    a.decodeAndUpcastDepotContactFromV1(
      { contact: { team: 'ops', zip_code: '123' } },
      hooks,
    ),
  ),
  { contact: { address: { zip: '123' }, tags: { team: 'ops' } } },
);
assert.equal(
  a.decodeAndUpcastDepotContactFromV1(
    { contact: { zip_code: '123', zone: 'future' } },
    hooks,
  ).ok,
  false,
);
const prototypeInput = a.decodeAndUpcastDepotExtraFromV1(
  {
    extra: JSON.parse('{"fixed":"x","__proto__":{"polluted":true},"tag":"ok"}'),
  },
  hooks,
);
assert.equal(
  prototypeInput.ok,
  false,
  'reserved keys must fail instead of disappearing during schema parsing',
);
const extra = take(
  a.decodeAndUpcastDepotExtraFromV1(
    { extra: { fixed: 'x', tag: 'ok' } },
    hooks,
  ),
).extra;
assert.deepEqual(extra.metadata, { tag: 'ok' });
assert.equal(Object.hasOwn(Object.prototype, 'polluted'), false);
assert.equal(
  a.decodeAndUpcastDepotExtraFromV1(
    { extra: { fixed: 'x', future: 'v3' } },
    hooks,
  ).ok,
  false,
);
const node = { children: [{ name: 'leaf' }], name: 'root' };
assert.deepEqual(take(a.decodeAndUpcastDepotInspectFromV1({ node }, hooks)), {
  node,
});
assert.equal(
  a.decodeAndUpcastDepotInspectFromV1(
    { node: { children: [{ name: 'leaf', unknown: 1 }], name: 'root' } },
    hooks,
  ).ok,
  false,
);
const bytes = new Uint8Array([0, 127, 255]);
assert.equal(
  take(a.decodeAndUpcastDepotBlobFromV1({ data: bytes }, hooks)).data,
  bytes,
);
assert.equal(
  a.decodeAndUpcastDepotBlobFromV1({ data: [0, 127, 255] }, hooks).ok,
  false,
);
assert.deepEqual(
  take(
    a.decodeAndUpcastDepotBindingsFromV1({ h: 'header', q: 'query' }, hooks),
  ),
  { h: 'header', q: 'query' },
);

const framework: a.FrameworkError = {
  error: new Error('transport'),
  kind: 'Framework',
};
assert.equal(
  take(a.downcastDepotGetErrorToV1(framework, storedValues, hooks)),
  framework,
);
assert.equal(exceptionHooks, 0);
const businessError = take(
  a.downcastDepotGetErrorToV1(
    new errors.Framework({ reason: 'locked' }),
    storedValues,
    hooks,
  ),
);
assert(businessError instanceof errors.v1$NotFound);
assert.equal(businessError.id, 'locked');
assert.equal(exceptionHooks, 2);
const denied = take(
  a.downcastDepotGetErrorToV1(
    new errors.Denied({ reason: 'denied' }),
    storedValues,
    hooks,
  ),
);
assert(denied instanceof errors.v1$NotFound);
assert.equal(
  exceptionHooks,
  4,
  'every adjacent error hook must execute exactly once',
);

const typedHead = {
  error: new errors.Denied({ reason: 'original denial' }),
  ok: false as const,
};
const typedRefusal = a.downcastDepotGetOutcomeToV1(typedHead, storedValues, {
  ...hooks,
  depotGetBusyErrorDowncastToV1: () => refuse('2026-01', 'Depot.get'),
});
assert.equal(typedRefusal.projected, false);
if (!typedRefusal.projected) {
  assert.equal(typedRefusal.head, typedHead);
  assert.equal(typedRefusal.head.error.reason, 'original denial');
}

const terminal = { ok: true as const, value: 'delivered' as const };
const failure = a.downcastDepotShipOutcomeToV1(terminal, hooks);
assert.equal(failure.projected, false);
if (!failure.projected) {
  assert.equal(failure.head, terminal);
  assert.equal(
    failure.error.version,
    '2026-01',
    'later downcast refusal must retain the original delivered outcome',
  );
}
const blocked: a.VersionAdapters = {
  ...hooks,
  parcelUpcastFromV2: () => ({
    error: {
      direction: 'upcast',
      reason: 'business decision refused',
      typePath: 'Parcel',
      version: '2026-04',
    },
    ok: false,
  }),
};
const beforeMiddleRefusal = hookCalls;
let callsAfterRefusal = 0;
const rejected = await a.dispatchDepotShip(
  'v1',
  { parcel: wireParcel },
  async () => {
    callsAfterRefusal++;
    return { ok: true, value: 'pending' };
  },
  blocked,
);
assert.equal(rejected.dispatched, false);
assert.equal(callsAfterRefusal, 0);
assert.equal(
  hookCalls,
  beforeMiddleRefusal + 1,
  'the earlier upcast ran before the middle refusal',
);
const headOnly = await a.dispatchDepotHeadOnly(
  'v3',
  {},
  async (_, context) => {
    assert.equal(context.originalVersionId, 'v3');
    return { ok: true, value: 'head' };
  },
  hooks,
);
assert(headOnly.dispatched && headOnly.projected && headOnly.value.ok);
assert.equal(headOnly.value.value, 'head');

const optionalHead = take(
  a.decodeAndUpcastDepotOptionalChangeFromV1(
    { value: { note: 'kept' } },
    hooks,
  ),
).value;
assert.deepEqual(
  take(a.downcastOptionalChange('v1', optionalHead, storedValues, hooks)),
  { note: 'kept' },
);
// The same instance with nothing in storage omits the member instead of inventing one.
assert.deepEqual(
  take(
    a.downcastOptionalChange('v1', { note: 'unknown' }, storedValues, hooks),
  ),
  {},
);
assert.equal(
  a.downcastOptionalChange('v1', optionalHead, storedValues, {
    ...hooks,
    optionalChangeDowncastToV1: () => refuse('2026-01', 'OptionalChange'),
  }).ok,
  false,
);
for (const decoder of [
  a.decodeAndUpcastDepotOptionalChangeFromV1,
  a.decodeAndUpcastDepotOptionalChangeFromV2,
  a.decodeAndUpcastDepotOptionalChangeFromV3,
]) {
  const absent = take(decoder({ value: {} }, hooks)).value;
  assert.equal(Object.hasOwn(absent, 'note'), false);
}
const headNode = take(
  a.decodeAndUpcastDepotInspectFromV3({ node: { name: 'root' } }, hooks),
).node;
assert.equal(Object.hasOwn(headNode, 'children'), false);

await runStoredJourney(hooks, storedValues, head, wireParcel);
await runHttpJourney(
  hooks,
  storedValues,
  storage.handlingNotes,
  wireParcel,
  head,
  () => hookCalls,
);
await runUpdateJourney(hooks, storedValues, storage.handlingNotes, head);
