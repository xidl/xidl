import assert from 'node:assert/strict';
import * as a from './versioned_adapters_single.adapters';

const hooks: a.VersionAdapters = {};
const decoded = a.decodeAndUpcastDepotShipFromOnly(
  { parcel: { id: 'p1' } },
  hooks,
);
assert(decoded.ok);
assert.equal(Object.hasOwn(decoded.value.parcel, 'note'), false);
let writes = 0;
const dispatch = async (wire: unknown) =>
  a.dispatchDepotShip(
    'only',
    wire,
    async (input, context) => {
      writes++;
      assert.deepEqual(context, {
        originalVersion: 'only',
        originalVersionId: 'only',
      });
      return { ok: true, value: input.parcel };
    },
    hooks,
  );
for (const parcel of [
  { note: 'missing required id' },
  { future: true, id: 'p1' },
  null,
]) {
  const invalid = await dispatch({ parcel });
  assert.equal(invalid.dispatched, false);
}
assert.equal(writes, 0);
const accepted = await dispatch({ parcel: { id: 'p1' } });
assert(accepted.dispatched && accepted.projected && accepted.value.ok);
assert.deepEqual(accepted.value.value, { id: 'p1' });
assert.equal(writes, 1);
const projected = a.downcastParcel('only', { id: 'p1' }, hooks);
assert(projected.ok);
assert.deepEqual(projected.value, { id: 'p1' });
console.log(
  'PASS: single-version identity, validation, optional absence, and exactly one dispatch',
);
