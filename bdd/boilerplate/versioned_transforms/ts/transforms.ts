import assert from 'node:assert/strict';
import { hooks } from './transforms_app';
import { runTransformsHttpJourney } from './transforms_http';
import type * as types from './versioned_transforms';
import * as a from './versioned_transforms.adapters';

/**
 * Storage keeps what older releases read; the Head instance carries the rest.
 * The generated chain never touches this map, it only receives readers.
 */
const rows = new Map<
  string,
  {
    createdAt: number | undefined;
    firstName: string | undefined;
    lastName: string | undefined;
    note: types.v2026_04_01.Note | undefined;
    status: string | undefined;
    recipient: string | undefined;
  }
>();
const stored: a.StoredValues = {
  profileCreatedAt: source => rows.get(source.id)?.createdAt,
  profileFirstName: source => rows.get(source.id)?.firstName,
  profileLastName: source => rows.get(source.id)?.lastName,
  profileNote: source => rows.get(source.id)?.note,
  profileRecipient: source => rows.get(source.id)?.recipient,
  profileStatus: source => rows.get(source.id)?.status,
};

// The retirement report: which release still reads which wire name.
assert.deepEqual(a.NEEDS, {
  Profile: {
    '2026-01-01': [
      'created_at',
      'first_name',
      'last_name',
      'note',
      'recipient',
      'status',
    ],
    '2026-04-01': ['created_at', 'first_name', 'last_name', 'note', 'status'],
    '2026-07-01': [],
  },
});

// E8 inverted: the upcast states the old behaviour (true), never the new
// default (false); the flat recipient becomes a route with no protocol,
// because which channel routes the message is the server's choice.
const oldest: types.v2026_01_01.Profile = {
  created_at: 1_700_000_000,
  first_name: 'Ann',
  id: 'p1',
  last_name: 'Lee',
  note: { text: 'prefers email' },
  recipient: 'anna@example.test',
  status: 'sent',
  title: 'founder',
};
rows.set('p1', {
  createdAt: oldest.created_at,
  firstName: oldest.first_name,
  lastName: oldest.last_name,
  note: oldest.note,
  recipient: oldest.recipient,
  status: oldest.status,
});
const upToV2 = a.upcastProfileFromV20260101(oldest, hooks);
assert(upToV2.ok, JSON.stringify(upToV2));
assert.equal(
  upToV2.value.fallback,
  true,
  'the old behaviour is stated, the new default is never applied',
);
assert.deepEqual(upToV2.value.to, {
  handle: 'anna@example.test',
});
assert.equal(Object.hasOwn(upToV2.value.to, 'protocol'), false);

// The second step merges into Head: E6 makes the timestamp deterministic,
// E3 merges the names mechanically and E7 grows the single note into a list.
const upToHead = a.upcastProfileFromV20260401(upToV2.value, hooks);
assert(upToHead.ok, JSON.stringify(upToHead));
assert.equal(upToHead.value.created_at_utc, '2023-11-14T22:13:20.000Z');
assert.equal(upToHead.value.display_name, 'Ann Lee');
assert.deepEqual(upToHead.value.notes, [{ text: 'prefers email' }]);
assert.equal(Object.hasOwn(upToHead.value, 'read_at'), false);

// E2 + E8 downward: the route reads back as the stored recipient with no
// invented protocol, and every stored fact returns untouched.
const downToV1 = a.downcastProfileToV20260101(upToV2.value, stored, hooks);
assert(downToV1.ok, JSON.stringify(downToV1));
assert.deepEqual(downToV1.value, {
  created_at: 1_700_000_000,
  first_name: 'Ann',
  id: 'p1',
  last_name: 'Lee',
  note: { text: 'prefers email' },
  recipient: 'anna@example.test',
  status: 'sent',
  title: 'founder',
});
// Round-trip rule one: read, upcast, downcast — the system is unchanged.
assert.deepEqual(
  a.downcastProfileToV20260101(upToV2.value, stored, hooks).ok &&
    downToV1.value,
  oldest,
);

// E5 upward: a retired enum value can never invent its timestamp.
const readUpcast = a.upcastProfileFromV20260101(
  { ...oldest, status: 'read' },
  hooks,
);
assert(!readUpcast.ok);
assert.equal(readUpcast.error.typePath, 'Profile.read_at');

// Head instance: a custom display name, two notes, the newest protocol value
// and a read time.
const head: types.v2026_07_01.Profile = {
  created_at_utc: '2026-06-01T12:00:00.000Z',
  display_name: 'Dr. Ann Lee',
  fallback: false,
  id: 'p1',
  notes: [{ text: 'prefers email' }, { text: 'vip' }],
  read_at: '2026-06-02T08:00:00.000Z',
  title: 'founder',
  to: { handle: 'anna@example.test', protocol: 'chat' },
};
rows.set('p1', {
  createdAt: 1_780_315_200,
  firstName: 'Ann',
  lastName: 'Lee',
  note: { text: 'prefers email' },
  recipient: 'anna@example.test',
  status: 'read',
});

// E4: the newest protocol value has no image in 2026-04; the marker stops the
// projection there.
const chatAtV2 = a.downcastProfileToV20260401(head, stored, hooks);
assert(!chatAtV2.ok);
assert.equal(chatAtV2.error.typePath, 'Route.protocol');

// E6 downward: the creation time floors to seconds, a declared loss.
const secondsOnly: types.v2026_07_01.Profile = {
  ...head,
  to: { handle: 'anna@example.test', protocol: 'relay' },
};
const downToV2 = a.downcastProfileToV20260401(secondsOnly, stored, hooks);
assert(downToV2.ok, JSON.stringify(downToV2));
assert.equal(downToV2.value.created_at, 1_780_315_200);
assert.equal(downToV2.value.first_name, 'Ann');
assert.equal(downToV2.value.last_name, 'Lee');
// E5 downward: the read time renders as the retired enum value.
assert.equal(downToV2.value.status, 'read');
// E7 downward: the list reads back as its first entry by declaration.
assert.deepEqual(downToV2.value.note, { text: 'prefers email' });

// The chain refuses at the first release that still declares the field: the
// multi-step downcast of the chat profile fails identically for 2026-01,
// because 2026-04 sits on the path and has no image for the value.
const chainRefusal = a.downcastProfile('v2026_01_01', head, stored, hooks);
assert(!chainRefusal.ok);
assert.equal(chainRefusal.error.typePath, 'Route.protocol');
// A route without the newest value walks the whole chain unchanged.
const downToV1Again = a.downcastProfileToV20260101(upToV2.value, stored, hooks);
assert(downToV1Again.ok);
assert.equal(downToV1Again.value.recipient, 'anna@example.test');
assert.equal(Object.hasOwn(downToV1Again.value, 'to'), false);

// E3 downward: the merged display name can only be served from the stored
// pieces; without them the projection refuses instead of guessing a split.
const anonymous: types.v2026_07_01.Profile = { ...head, id: 'p-anon' };
const splitRefusal = a.downcastProfileToV20260401(anonymous, stored, hooks);
assert(!splitRefusal.ok);
// The chain stops at the first missing fact, and the decision asks in the
// declaration order: the creation time is missing before the names are.
assert.equal(splitRefusal.error.typePath, 'Profile.created_at');
const namesRefusal = a.downcastProfileToV20260401(
  anonymous,
  {
    ...stored,
    profileCreatedAt: () => 0,
  },
  hooks,
);
assert(!namesRefusal.ok);
assert.equal(namesRefusal.error.typePath, 'Profile.first_name');

// E7 without storage: nothing stored means nothing served, an explicit OMIT
// instead of a fabricated note.
const noteless = a.downcastProfileToV20260401(
  secondsOnly,
  {
    ...stored,
    profileNote: () => undefined,
  },
  hooks,
);
assert(noteless.ok);
assert.equal(Object.hasOwn(noteless.value, 'note'), false);

// Round-trip rule two: writing exactly the served view back must not erase
// what the writer cannot see. That merge lives in the application, and the
// HTTP journey pins it end to end.
await runTransformsHttpJourney();

console.log(
  'PASS: every transform shape upcasts, downcasts and round-trips without inventing facts',
);
