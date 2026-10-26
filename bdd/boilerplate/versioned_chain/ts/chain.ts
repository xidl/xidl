import assert from 'node:assert/strict';
import { runChainHttpJourney } from './chain_http';
import type * as types from './versioned_chain';
import * as a from './versioned_chain.adapters';

const ok = <T>(value: T): a.VersionedResult<T> => ({ ok: true, value });

/**
 * Storage only moves forward: each booking row keeps what older releases
 * still read. The generated chain never touches this; it receives readers.
 */
const agendaTexts = new Map<string, string>();
const rosters = new Map<string, string[]>();
const calls = { downToV1: 0, downToV2: 0, upFromV1: 0, upFromV2: 0 };

const stored: a.StoredValues = {
  bookingAgenda: source => agendaTexts.get(source.id),
  bookingOwner: source => source.organizer,
  bookingRoster: source => rosters.get(source.id),
};

const refuse = (
  typePath: string,
  reason: string,
): a.VersionedResult<never> => ({
  error: { direction: 'downcast', reason, typePath, version: 'n/a' },
  ok: false,
});

const hooks: a.VersionAdapters = {
  bookingDowncastToV20260601: (source, storedValues) => {
    calls.downToV1++;
    const agenda = storedValues.bookingAgenda(source);
    if (agenda === undefined) {
      return refuse(
        'Booking.agenda',
        `no stored agenda for booking ${source.id}`,
      );
    }
    return ok({ agenda });
  },
  bookingDowncastToV20261001: (source, storedValues) => {
    calls.downToV2++;
    const roster = storedValues.bookingRoster(source);
    if (roster === undefined) {
      return refuse(
        'Booking.roster',
        `no stored roster for booking ${source.id}`,
      );
    }
    return ok({ owner: storedValues.bookingOwner(source) ?? a.OMIT, roster });
  },
  // The oldest release packs the plan into one text block. The application
  // states what the Head contract expects; the retired agenda is acknowledged.
  bookingUpcastFromV20260601: source => {
    calls.upFromV1++;
    const separator = source.agenda.indexOf(';');
    const title = source.agenda.slice(0, Math.max(separator, 0));
    const minutes =
      separator === -1 ? undefined : source.agenda.slice(separator + 1);
    // The duration must fit the contract's integer range: every language
    // refuses the same agenda encodings.
    const duration = minutes === undefined ? Number.NaN : Number(minutes);
    if (
      separator === -1 ||
      minutes === undefined ||
      !/^\d+$/.test(minutes) ||
      !Number.isSafeInteger(duration) ||
      duration > 0x7fffffff
    ) {
      return {
        error: {
          direction: 'upcast' as const,
          reason: `unreadable agenda for booking ${source.id}`,
          typePath: 'Booking.agenda',
          version: '2026-06-01',
        },
        ok: false,
      };
    }
    return ok({
      agenda: a.IGNORED,
      items: [{ minutes: duration, title }],
    });
  },
  bookingUpcastFromV20261001: source => {
    calls.upFromV2++;
    return ok({ organizer: source.owner, owner: a.IGNORED, roster: a.IGNORED });
  },
};

const HEAD: types.v2027_01_01.Booking = {
  id: 'b1',
  items: [{ minutes: 60, title: 'Kickoff' }],
  organizer: 'user-9',
  title: 'Kickoff',
};

// The retirement report: which release still reads which wire name.
assert.deepEqual(a.NEEDS, {
  Booking: {
    '2026-06-01': ['agenda', 'owner', 'roster'],
    '2026-10-01': ['owner', 'roster'],
    '2027-01-01': [],
  },
});

agendaTexts.set('b1', 'Kickoff;60');
rosters.set('b1', ['user-1', 'user-2']);

// Head -> 2026-10-01: the roster returns from storage; organizer keeps the
// role under its older source name through the explicit application hook.
const toV2 = a.downcastBooking('v2026_10_01', HEAD, stored, hooks);
assert(toV2.ok, JSON.stringify(toV2));
assert.deepEqual(toV2.value.roster, ['user-1', 'user-2']);
assert.equal(toV2.value.owner, 'user-9');
assert.deepEqual(toV2.value.items, HEAD.items);
assert.equal(calls.downToV2, 1, 'one aggregate owner/roster decision ran');

// Head -> 2026-06-01 in one call walks both steps.
const toV1 = a.downcastBooking('v2026_06_01', HEAD, stored, hooks);
assert(toV1.ok, JSON.stringify(toV1));
assert.equal(
  toV1.value.agenda,
  'Kickoff;60',
  'the oldest release reads the stored agenda',
);
assert.deepEqual(toV1.value.roster, ['user-1', 'user-2']);
assert.equal(toV1.value.owner, 'user-9');
assert.equal(calls.downToV1, 1);
assert.equal(
  calls.upFromV1 + calls.upFromV2,
  0,
  'downcasting never runs upcast hooks',
);

// A row that storage never kept cannot be served: information only gets
// dropped, never invented. The chain stops at the first missing fact.
const orphan: types.v2027_01_01.Booking = { ...HEAD, id: 'b-missing' };
const missingV2 = a.downcastBooking('v2026_10_01', orphan, stored, hooks);
assert(!missingV2.ok);
assert.equal(missingV2.error.typePath, 'Booking.roster');
const missingV1 = a.downcastBooking('v2026_06_01', orphan, stored, hooks);
assert(!missingV1.ok);
assert.equal(
  missingV1.error.typePath,
  'Booking.roster',
  'the chain stops at the first missing fact',
);
const missingAgenda = a.downcastBooking(
  'v2026_06_01',
  orphan,
  { ...stored, bookingAgenda: () => undefined, bookingRoster: () => [] },
  hooks,
);
assert(!missingAgenda.ok);
assert.equal(missingAgenda.error.typePath, 'Booking.agenda');

// The oldest caller upgrades: the retired agenda is acknowledged, never
// forwarded, and the unpacked entries come from the application's decision.
const v1Booking: types.v2026_06_01.Booking = {
  agenda: 'Kickoff;60',
  id: 'b1',
  owner: 'user-9',
  roster: ['user-1', 'user-2'],
  title: 'Kickoff',
};
const upToV2 = a.upcastBookingFromV20260601(v1Booking, hooks);
assert(upToV2.ok, JSON.stringify(upToV2));
assert.equal(calls.upFromV1, 1);
assert.deepEqual(upToV2.value.items, [{ minutes: 60, title: 'Kickoff' }]);
assert.equal(upToV2.value.owner, 'user-9');
assert.deepEqual(upToV2.value.roster, ['user-1', 'user-2']);

// Write-then-read is a fixed point: the oldest wire shape survives the trip
// to Head and back because storage kept the retired facts.
const toHead = a.upcastBookingFromV20261001(upToV2.value, hooks);
assert(toHead.ok, JSON.stringify(toHead));
assert.equal(calls.upFromV2, 1);
assert.equal(toHead.value.organizer, 'user-9');
const roundTrip = a.downcastBooking('v2026_06_01', toHead.value, stored, hooks);
assert(roundTrip.ok, JSON.stringify(roundTrip));
assert.deepEqual(roundTrip.value, v1Booking);

// An ambiguous text block is refused, not guessed: the sample encoding cannot
// represent a title that contains the separator.
const ambiguous = a.upcastBookingFromV20260601(
  { ...v1Booking, agenda: 'A;B;45', id: 'b-amb' },
  hooks,
);
assert(!ambiguous.ok);
assert.equal(ambiguous.error.typePath, 'Booking.agenda');
assert.equal(
  calls.upFromV1,
  2,
  'the refusal still went through the application hook',
);

// Malformed old-version requests fail validation before any hook runs.
const invalid = a.decodeAndUpcastSchedulerBookFromV20260601(
  { booking: { id: 'b2', roster: [], title: 'No agenda' } },
  hooks,
);
assert(!invalid.ok);
assert.equal('kind' in invalid.error && invalid.error.kind, 'Validation');
assert.equal(calls.upFromV1, 2, 'validation precedes every hook');

// An absent optional predecessor stays absent: the rename copies what is
// there and invents nothing when the caller omitted it.
const ownerless = a.upcastBookingFromV20260601(
  { ...v1Booking, owner: undefined },
  hooks,
);
assert(ownerless.ok, JSON.stringify(ownerless));
assert.equal(
  ownerless.value.owner,
  undefined,
  'the rename carries absence, never a fabricated owner',
);
const ownerlessHead = a.upcastBookingFromV20261001(ownerless.value, hooks);
assert(ownerlessHead.ok, JSON.stringify(ownerlessHead));
assert.equal(
  ownerlessHead.value.organizer,
  undefined,
  'Head sees no organizer when no owner was ever named',
);

console.log(
  'PASS: booking chain keeps storage facts, pure conversions and application renames',
);

await runChainHttpJourney();
