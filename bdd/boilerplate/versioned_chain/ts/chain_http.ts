import assert from 'node:assert/strict';
import type { AddressInfo } from 'node:net';
import type { BookingAppState } from './chain_app';
import { createBookingApp } from './chain_app';
import type * as types from './versioned_chain';

export async function runChainHttpJourney(): Promise<void> {
  const state: BookingAppState = {
    ledger: new Map(),
    rows: new Map(),
    writes: 0,
  };
  const server = createBookingApp(state);
  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
  try {
    const base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
    const post = (
      versionHeader: string | undefined,
      body: unknown,
      key: string,
    ) =>
      fetch(`${base}/bookings`, {
        body: JSON.stringify(body),
        headers: {
          'content-type': 'application/json',
          ...(versionHeader === undefined
            ? {}
            : { 'x-booking-version': versionHeader }),
          'idempotency-key': key,
        },
        method: 'POST',
      });
    const get = (versionHeader: string | undefined, id: string) =>
      fetch(`${base}/bookings?id=${id}`, {
        headers:
          versionHeader === undefined
            ? {}
            : { 'x-booking-version': versionHeader },
      });

    // The oldest release writes; the response projects back to its own
    // contract, and the handler only ever saw the upcast Head command.
    const v1Body = {
      agenda: 'Kickoff;60',
      id: 'b1',
      roster: ['user-1'],
      title: 'Kickoff',
    };
    const oldest = await post('2026-06-01', v1Body, 'kickoff');
    assert.equal(oldest.status, 200);
    assert.equal(oldest.headers.get('x-booking-version'), '2026-06-01');
    const oldestView = await oldest.json();
    assert.equal(oldestView.agenda, 'Kickoff;60');
    assert.equal(state.writes, 1);
    // Head reads the same booking: the stored agenda unpacked by the hook.
    const asHead = (await (
      await get(undefined, 'b1')
    ).json()) as types.v2027_01_01.Booking;
    assert.deepEqual(asHead.items, [{ minutes: 60, title: 'Kickoff' }]);

    // Negotiation: a mid-patch date resolves to the newest release not later
    // than it; earlier or malformed headers are a 400, never a guess.
    const negotiation: Array<[string | undefined, string, number]> = [
      [undefined, '2027-01-01', 404],
      ['2026-08-15', '2026-06-01', 404],
      ['2026-10-01', '2026-10-01', 404],
      ['2026-05-31', 'unresolved', 400],
      ['next-tuesday', 'unresolved', 400],
    ];
    for (const [header, echoed, status] of negotiation) {
      const probe = await get(header, 'missing');
      assert.equal(
        probe.status,
        status,
        `${header ?? '<absent>'} -> ${status}`,
      );
      assert.equal(probe.headers.get('x-booking-version'), echoed);
    }

    // Cross-release idempotency: the same meaning sent from Head with the
    // same key replays the first booking; a different command conflicts.
    const replay = await post(
      '2027-01-01',
      {
        id: 'b1',
        items: [{ minutes: 60, title: 'Kickoff' }],
        title: 'Kickoff',
      },
      'kickoff',
    );
    assert.equal(replay.status, 200);
    const replayView = await replay.json();
    const headNow = await (await get(undefined, 'b1')).json();
    assert.deepEqual(
      replayView,
      headNow,
      'the retry returns the original outcome',
    );
    assert.equal(
      state.writes,
      1,
      'the fingerprint lives on the upcast command',
    );
    const conflict = await post(
      '2027-01-01',
      {
        id: 'b1',
        items: [{ minutes: 30, title: 'Changed' }],
        title: 'Changed',
      },
      'kickoff',
    );
    assert.equal(conflict.status, 409, 'same key, different command');
    assert.equal(state.writes, 1);

    // Behaviour switch: Head refuses a blank title; the oldest release never
    // had that behaviour and accepts the same request.
    const blankV1 = { agenda: ' ;0', id: 'b-blank', roster: [], title: '   ' };
    assert.equal((await post('2026-06-01', blankV1, 'blank-v1')).status, 200);
    const blankHead = await post(
      undefined,
      {
        id: 'b-blank-head',
        items: [{ minutes: 0, title: '   ' }],
        title: '   ',
      },
      'blank-head',
    );
    assert.equal(blankHead.status, 422);
    assert.equal(state.writes, 2, 'only the accepted bookings were written');

    // Read your writes: the oldest caller's facts come straight back, because
    // the application stored them from its own wire when the call arrived.
    const v1Read = await get('2026-06-01', 'b1');
    assert.equal(v1Read.status, 200);
    const v1View = await v1Read.json();
    assert.equal(v1View.agenda, 'Kickoff;60');
    assert.deepEqual(v1View.roster, ['user-1']);
    assert.equal(Object.hasOwn(v1View, 'items'), false);

    // A booking created at Head never had an agenda fact; the oldest release
    // cannot be served one, and the conversion refuses instead of inventing.
    const headCreated = await post(
      undefined,
      {
        id: 'b-head',
        items: [{ minutes: 15, title: 'Ad hoc' }],
        title: 'Ad hoc',
      },
      'ad-hoc',
    );
    assert.equal(headCreated.status, 200);
    const headCreatedV1 = await get('2026-06-01', 'b-head');
    assert.equal(headCreatedV1.status, 422);

    // An ambiguous text block is refused, not guessed: a title containing the
    // separator makes the old encoding undecodable, so the upcast refuses
    // before any handler runs.
    const ambiguous = await post(
      '2026-06-01',
      { agenda: 'A;B;45', id: 'b-amb', roster: [], title: 'Sync' },
      'ambiguous',
    );
    assert.equal(ambiguous.status, 400);
    assert.equal(state.writes, 3, 'the refused write never committed');

    // The behaviour switch gates writes, not history: a blank title accepted
    // by the oldest release stays readable at Head until the product decides
    // otherwise.
    const blankAtHead = await get(undefined, 'b-blank');
    assert.equal(blankAtHead.status, 200);
    assert.equal((await blankAtHead.json()).title, '   ');

    // Two keys for one id are two commands: both succeed and the later write
    // wins. Uniqueness of ids is the application's rule, not the chain's.
    const firstId = await post(
      '2026-06-01',
      { agenda: 'First;10', id: 'b-dupe', roster: [], title: 'First' },
      'dupe-1',
    );
    const secondId = await post(
      undefined,
      {
        id: 'b-dupe',
        items: [{ minutes: 20, title: 'Second' }],
        title: 'Second',
      },
      'dupe-2',
    );
    assert.equal(firstId.status, 200);
    assert.equal(secondId.status, 200);
    assert.equal(
      (await (await get(undefined, 'b-dupe')).json()).title,
      'Second',
    );
    assert.equal(state.writes, 5);
    const headRead = await get(undefined, 'b1');
    const headView = await headRead.json();
    assert.equal(headView.agenda, undefined);

    console.log(
      'PASS: booking chain negotiates releases, fingerprints across releases and switches behaviour by date',
    );
  } finally {
    server.closeAllConnections();
    await new Promise<void>((resolve, fail) =>
      server.close(error => (error ? fail(error) : resolve())),
    );
  }
}
