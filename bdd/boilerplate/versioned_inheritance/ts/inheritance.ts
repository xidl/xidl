import assert from 'node:assert/strict';
import type * as types from './versioned_inheritance';
import * as a from './versioned_inheritance.adapters';

const ok = <T>(value: T): a.VersionedResult<T> => ({ ok: true, value });
const take = <T>(result: a.VersionedResult<T>): T => {
  assert(result.ok, JSON.stringify(result));
  return result.value;
};
const refusal = (
  version: string,
  direction: 'upcast' | 'downcast',
  typePath: string,
): a.VersionedResult<never> => ({
  error: {
    direction,
    reason: 'address has no faithful representation',
    typePath,
    version,
  },
  ok: false,
});
let current: types.v3.shipping.Address = {
  country_code_v2: 'US',
  id: 'a1',
  previous: [{}],
  priority: 1,
  signature_required: true,
  street: 'Shared street',
  tenant: 'acme',
};
const history = [{ note: undefined as string | undefined, reference: 'A-1' }];
const stored = (): a.StoredValues => {
  // Bind response readers to the same commit as its Head, before dispatch awaits.
  const facts = history.at(-1);
  assert(facts);
  return {
    shippingAddressCountryCode: source => source.country_code_v2,
    shippingAddressDeliveryNote: source =>
      source.tenant === 'acme' && source.id === 'a1' ? facts.note : undefined,
    shippingAddressDeliveryReference: source =>
      source.tenant === 'acme' && source.id === 'a1'
        ? facts.reference
        : undefined,
    shippingAddressLine: source => source.street,
  };
};
const hooks: a.VersionAdapters = {
  shippingAddressDowncastToV1: (source, readers) => {
    const line = readers.shippingAddressLine(source);
    if (line === undefined)
      return refusal('2026-04-01', 'downcast', 'shipping::Address.line');
    const country = readers.shippingAddressCountryCode(source);
    if (country === undefined)
      return refusal('2026-04-01', 'downcast', 'shipping::Address.countryCode');
    return ok({ country_code: country, line });
  },
  shippingAddressDowncastToV2: (source, readers) => {
    const reference = readers.shippingAddressDeliveryReference(source);
    if (reference === undefined)
      return refusal(
        '2026-07-01',
        'downcast',
        'shipping::Address.deliveryReference',
      );
    return ok({
      delivery_note: readers.shippingAddressDeliveryNote(source) ?? a.OMIT,
      delivery_reference: reference,
    });
  },
  shippingChangeUpcastFromV1: source => {
    const code = source.country_code;
    if (code !== undefined && code !== 'US' && code !== 'CA')
      return refusal('2026-01-01', 'upcast', 'shipping::Change.countryCode');
    return ok({
      country_code: a.IGNORED,
      country_code_v2: code,
      line: a.IGNORED,
      stored: source.line === undefined ? {} : { line: source.line },
      street: source.line,
    });
  },
  shippingChangeUpcastFromV2: (source, head) =>
    ok({
      delivery_note: a.IGNORED,
      delivery_reference: a.IGNORED,
      signature_required: head.signature_required ?? false,
      stored: {
        ...(source.delivery_reference === undefined
          ? {}
          : { delivery_reference: source.delivery_reference }),
        ...(source.delivery_note === undefined
          ? {}
          : { delivery_note: source.delivery_note }),
      },
    }),
};

const handler = async (
  input: { patch: types.v3.shipping.Change },
  _: a.VersionContext,
  bag: a.ShippingAddressesUpdateStored,
) => {
  const previous = history.at(-1);
  assert(previous);
  const next = {
    note: bag.delivery_note ?? previous.note,
    reference: bag.delivery_reference ?? previous.reference,
  };
  if (JSON.stringify(next) !== JSON.stringify(previous)) history.push(next);
  current = {
    ...current,
    ...Object.fromEntries(
      Object.entries(input.patch).filter(([, value]) => value !== undefined),
    ),
    street: input.patch.street ?? bag.line ?? current.street,
  };
  return { ok: true as const, stored: stored(), value: current };
};
const envelope = (patch: unknown) => ({ id: 'a1', patch, tenant: 'acme' });
const first = await a.dispatchshipping_AddressesUpdateFromV1(
  envelope({
    countryCode: 'CA',
    deliveryNote: '',
    deliveryReference: 'A-NEW',
    line: 'New street',
    priority: 0,
  }),
  current,
  handler,
  hooks,
);
assert(first.dispatched && first.projected && first.value.ok);
assert.equal(first.value.value.delivery_note, '');
assert.equal(first.value.value.delivery_reference, 'A-NEW');
assert.equal(first.value.value.line, 'New street');
assert.equal(first.value.value.priority, 0);
assert.deepEqual(first.value.value.previous, [{}]);
assert.equal(current.signature_required, true);
assert.equal(Object.hasOwn(current, 'delivery_reference'), false);
assert.deepEqual(
  history[0],
  { note: undefined, reference: 'A-1' },
  'the stored historical row was never rewritten',
);
assert.deepEqual(a.NEEDS['shipping::Address'], {
  '2026-01-01': ['countryCode', 'deliveryNote', 'deliveryReference', 'line'],
  '2026-04-01': ['deliveryNote', 'deliveryReference'],
  '2026-07-01': [],
});
// Restating the narrow view preserves both Head and facts; writing reads back exactly.
const before = structuredClone({ current, history });
const view = take(a.downcastShippingAddress('v1', current, stored(), hooks));
const restated = await a.dispatchshipping_AddressesUpdateFromV1(
  envelope({
    countryCode: view.country_code,
    deliveryNote: view.delivery_note,
    deliveryReference: view.delivery_reference,
    line: view.line,
    priority: view.priority,
  }),
  current,
  handler,
  hooks,
);
assert(restated.dispatched && restated.projected && restated.value.ok);
assert.deepEqual(restated.value.value, view);
assert.deepEqual({ current, history }, before);
const middle = take(a.downcastShippingAddress('v2', current, stored(), hooks));
assert.equal(middle.street, view.line);
assert.equal(middle.delivery_note, view.delivery_note);
let calls = 0;
const tracked = {
  ...hooks,
  shippingChangeUpcastFromV2: (
    ...args: Parameters<a.VersionAdapters['shippingChangeUpcastFromV2']>
  ) => {
    calls++;
    return hooks.shippingChangeUpcastFromV2(...args);
  },
};
for (const patch of [
  { countryCodeV2: 'Atlantis' },
  { signatureRequired: false },
  { base: { deliveryNote: 'hidden' } },
]) {
  const result = a.decodeAndUpcastshipping_AddressesUpdateFromV2(
    envelope(patch),
    current,
    tracked,
  );
  assert.equal(result.ok, false);
}
assert.equal(calls, 0, 'wire rejection happens before inherited member hooks');
const invalid = await a.dispatchshipping_AddressesUpdateFromV1(
  envelope({ countryCode: 'Atlantis' }),
  current,
  handler,
  hooks,
);
assert.equal(invalid.dispatched, false);
assert.deepEqual(
  { current, history },
  before,
  'a refused conversion cannot reach the shared write',
);
const missing = a.downcastShippingAddress(
  'v1',
  { ...current, id: 'orphan' },
  stored(),
  hooks,
);
assert(!missing.ok);
assert.equal(missing.error.typePath, 'shipping::Address.deliveryReference');
assert.equal(missing.error.version, '2026-07-01');
assert.equal(
  take(
    a.downcastShippingAddress(
      'v3',
      { ...current, id: 'orphan' },
      stored(),
      hooks,
    ),
  ).id,
  'orphan',
);
const empty = await a.dispatchshipping_AddressesUpdateFromV1(
  envelope({ line: '' }),
  current,
  handler,
  hooks,
);
assert(empty.dispatched && empty.projected && empty.value.ok);
assert.equal(empty.value.value.line, '');
assert.equal(current.street, '');
assert.equal(Object.hasOwn(current, 'line'), false);
const afterEmpty = structuredClone({ current, history });
const omitted = await a.dispatchshipping_AddressesUpdateFromV1(
  envelope({}),
  current,
  handler,
  hooks,
);
assert(omitted.dispatched && omitted.projected && omitted.value.ok);
assert.equal(omitted.value.value.line, '');
assert.deepEqual({ current, history }, afterEmpty);
const missingLine = a.downcastShippingAddress(
  'v1',
  current,
  { ...stored(), shippingAddressLine: () => undefined },
  hooks,
);
assert(!missingLine.ok);
assert.equal(missingLine.error.typePath, 'shipping::Address.line');
assert.equal(missingLine.error.version, '2026-04-01');
const [left, right] = await Promise.all(
  ['LEFT', 'RIGHT'].map(reference =>
    a.dispatchshipping_AddressesUpdateFromV1(
      envelope({ deliveryReference: reference, line: `${reference} street` }),
      current,
      handler,
      hooks,
    ),
  ),
);
assert(left.dispatched && left.projected && left.value.ok);
assert(right.dispatched && right.projected && right.value.ok);
assert.deepEqual(
  [left.value.value.line, left.value.value.delivery_reference],
  ['LEFT street', 'LEFT'],
);
assert.deepEqual(
  [right.value.value.line, right.value.value.delivery_reference],
  ['RIGHT street', 'RIGHT'],
);
console.log('PASS: TypeScript inherited address request and storage decisions');
