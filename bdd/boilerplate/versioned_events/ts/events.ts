import assert from 'node:assert/strict';
import type * as types from './versioned_events';
import * as a from './versioned_events.adapters';

const ok = <T>(value: T): a.VersionedResult<T> => ({ ok: true, value });
const take = <T>(
  result: { ok: true; value: T } | { ok: false; error: unknown },
): T => {
  if (!result.ok) throw new Error(JSON.stringify(result.error));
  return result.value;
};

/** Storage only moves forward: every emitted event keeps its exact id. */
const state = { writes: 0, events: [] as types.v2026_07_01.Event[] };

const hooks: a.VersionAdapters = {
  // Both halves of the split serve the old name: one delivered push and one
  // read push reach an old subscriber as two distinct updated events.
  eventKindDowncastToV20260401: () => ok('updated'),
  // The old name keeps one half through the upcast: the application states
  // which half owns the name; the other half is only reachable at Head.
  eventKindUpcastFromV20260401: () => ok('delivered'),
};

/** The Head handler: emit appends, feed replays the committed order. */
const emit = async (
  version: 'v2026_04_01' | 'v2026_07_01',
  event: { id: string; kind: string; payload: string },
) =>
  a.dispatchNotifierEmit(
    version,
    { event },
    async request => {
      // The handler stores the decoded Head command, never the raw wire.
      const committed = request.event;
      state.writes++;
      state.events.push(committed);
      return { ok: true, value: committed };
    },
    hooks,
  );

const feed = async (version: 'v2026_04_01' | 'v2026_07_01') =>
  a.dispatchNotifierFeed(
    version,
    {},
    async () => ({ ok: true, value: state.events }),
    hooks,
  );

// Head emits both halves: the broad name retired, two precise ones born.
const delivered = await emit('v2026_07_01', {
  id: 'e1',
  kind: 'delivered',
  payload: 'receipt ready',
});
assert(delivered.dispatched && delivered.projected && delivered.value.ok);
assert.equal(delivered.value.value.kind, 'delivered');
const read = await emit('v2026_07_01', {
  id: 'e2',
  kind: 'read',
  payload: 'opened twice',
});
assert(read.dispatched && read.projected && read.value.ok);
assert.equal(read.value.value.kind, 'read');
assert.equal(state.writes, 2);

// An old subscriber reads the feed: both halves arrive under the retired
// name, and the two pushes stay distinct through their own ids.
const oldFeed = await feed('v2026_04_01');
assert(oldFeed.dispatched && oldFeed.projected && oldFeed.value.ok);
assert.deepEqual(
  oldFeed.value.value.map(event => ({ id: event.id, kind: event.kind })),
  [
    { id: 'e1', kind: 'updated' },
    { id: 'e2', kind: 'updated' },
  ],
  'one delivered and one read push reach the old subscriber as two updates',
);

// An old caller emits through the retired name: the upcast states the half
// that owns the name, and the old response reads the same name back.
const replayed = await emit('v2026_04_01', {
  id: 'e3',
  kind: 'updated',
  payload: 'legacy sender',
});
assert(replayed.dispatched && replayed.projected && replayed.value.ok);
assert.equal(replayed.value.value.kind, 'updated');
assert.equal(state.events[2].kind, 'delivered', 'storage keeps the stated half');
assert.equal(state.writes, 3);

// The declared loss: the half that does not own the name is only reachable
// at Head — an old emit can never produce a read event.
assert.equal(
  state.events.some(event => event.kind === 'read' && event.id === 'e3'),
  false,
);

// A value the old contract never declared is refused before any hook: the
// split halves are not new names for old callers to guess.
const refused = await a.dispatchNotifierEmit(
  'v2026_04_01',
  { event: { id: 'e4', kind: 'delivered', payload: 'future name' } },
  async () => {
    throw new Error('validation refusals must never reach the handler');
  },
  hooks,
);
assert.equal(refused.dispatched, false);
assert.equal(state.writes, 3);

// Round-trip rule: write through the old name, read back the same name.
const roundTrip = await feed('v2026_04_01');
assert(roundTrip.dispatched && roundTrip.projected && roundTrip.value.ok);
assert.equal(roundTrip.value.value[2].id, 'e3');
assert.equal(roundTrip.value.value[2].kind, 'updated');

console.log('PASS: event split keeps old names, distinct ids and one stated half');
