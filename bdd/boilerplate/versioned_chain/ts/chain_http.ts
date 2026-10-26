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
      key?: string,
    ) =>
      fetch(`${base}/bookings`, {
        body: typeof body === 'string' ? body : JSON.stringify(body),
        headers: {
          'content-type': 'application/json',
          ...(versionHeader === undefined
            ? {}
            : { 'x-booking-version': versionHeader }),
          ...(key === undefined ? {} : { 'idempotency-key': key }),
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
      owner: 'user-9',
      roster: ['user-1'],
      title: 'Kickoff',
    };
    const oldest = await post('2026-06-01', v1Body, 'kickoff');
    assert.equal(oldest.status, 200);
    assert.equal(oldest.headers.get('x-booking-version'), '2026-06-01');
    const oldestView = await oldest.json();
    assert.equal(oldestView.agenda, 'Kickoff;60');
    assert.equal(oldestView.owner, 'user-9');
    assert.equal(state.writes, 1);
    // Head reads the same booking: the stored agenda unpacked by the hook
    // and owner under its Head name.
    const asHead = (await (
      await get(undefined, 'b1')
    ).json()) as types.v2027_01_01.Booking;
    assert.deepEqual(asHead.items, [{ minutes: 60, title: 'Kickoff' }]);
    assert.equal(asHead.organizer, 'user-9');

    // This application's example policy: a mid-patch date resolves to the newest
    // release not later than it. Invalid or too-early headers return 400.
    const negotiation: Array<[string | undefined, string, number]> = [
      [undefined, '2027-01-01', 404],
      ['2026-08-15', '2026-06-01', 404],
      ['2026-10-01', '2026-10-01', 404],
      ['2026-12-31', '2026-10-01', 404],
      ['2026-05-31', 'unresolved', 400],
      ['next-tuesday', 'unresolved', 400],
      ['2026-13-01', 'unresolved', 400],
      ['2026-02-30', 'unresolved', 400],
      ['2026-6-01', 'unresolved', 400],
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
    // A POST under an unsupported header is refused before any handler runs.
    const unresolvedPost = await post('2026-05-31', v1Body, 'unresolved');
    assert.equal(unresolvedPost.status, 400);
    assert.equal(unresolvedPost.headers.get('x-booking-version'), 'unresolved');
    assert.equal(state.writes, 1);

    // Transport-level refusals never reach a handler: every POST names an
    // idempotency key and carries parseable JSON; poison keys change nothing.
    assert.equal((await post('2026-06-01', v1Body)).status, 400);
    assert.equal(
      (await post('2026-06-01', 'not-json{', 'bad-json')).status,
      400,
    );
    const polluted = JSON.parse(
      '{"agenda":"Kickoff;5","id":"b-pollute","roster":[],"title":"Sync","__proto__":{"polluted":true}}',
    ) as Record<string, unknown>;
    assert.equal((await post('2026-06-01', polluted, 'pollute')).status, 400);
    assert.equal(state.writes, 1, 'no refused request reached a handler');
    assert.equal(
      ({} as Record<string, unknown>).polluted,
      undefined,
      'Object.prototype stays unpolluted',
    );
    assert.equal(state.rows.has('b-pollute'), false);

    // Cross-release idempotency: the same meaning sent from any release with
    // the same key replays the first booking; a different command conflicts.
    const replay = await post(
      '2027-01-01',
      {
        id: 'b1',
        items: [{ minutes: 60, title: 'Kickoff' }],
        organizer: 'user-9',
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
    // The same meaning replayed from the middle release projects the cached
    // outcome into that caller's contract.
    const middleReplay = await post(
      '2026-10-01',
      {
        id: 'b1',
        items: [{ minutes: 60, title: 'Kickoff' }],
        owner: 'user-9',
        roster: ['ignored-by-the-chain'],
        title: 'Kickoff',
      },
      'kickoff',
    );
    assert.equal(middleReplay.status, 200);
    const middleView = (await middleReplay.json()) as types.v2026_10_01.Booking;
    assert.deepEqual(middleView.items, [{ minutes: 60, title: 'Kickoff' }]);
    assert.deepEqual(middleView.roster, ['user-1'], 'storage answers v2');
    assert.equal(state.writes, 1);
    // A command that differs only in facts the chain retires is still the same
    // meaning: it replays, and storage keeps the facts of the committed one.
    const rosterReplay = await post(
      '2026-06-01',
      { ...v1Body, roster: ['user-9', 'user-8'] },
      'kickoff',
    );
    assert.equal(rosterReplay.status, 200);
    assert.equal(state.writes, 1, 'the replay never rewrote storage');
    const rosterView = await (await get('2026-06-01', 'b1')).json();
    assert.deepEqual(
      rosterView.roster,
      ['user-1'],
      'the replayed command left the stored facts alone',
    );
    const conflict = await post(
      '2027-01-01',
      {
        id: 'b1',
        items: [{ minutes: 30, title: 'Changed' }],
        organizer: 'user-9',
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
    // Replaying the accepted command at Head answers before the switch: the
    // original decision stands regardless of when the retry arrives.
    const blankReplay = await post(
      '2027-01-01',
      {
        id: 'b-blank',
        items: [{ minutes: 0, title: ' ' }],
        title: '   ',
      },
      'blank-v1',
    );
    assert.equal(blankReplay.status, 200);
    assert.equal(state.writes, 2, 'a replay is never a second write');
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
    // the application stored them from its own wire when the call committed.
    const v1Read = await get('2026-06-01', 'b1');
    assert.equal(v1Read.status, 200);
    const v1View = await v1Read.json();
    assert.equal(v1View.agenda, 'Kickoff;60');
    assert.deepEqual(v1View.roster, ['user-1']);
    assert.equal(v1View.owner, 'user-9');
    assert.equal(Object.hasOwn(v1View, 'items'), false);

    // A booking created at Head never had an agenda fact; neither release
    // that still reads one can be served: the conversion refuses instead of
    // inventing, and the response carries the refusal reason.
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
    // The chain stops at the first missing fact: the roster step precedes the
    // agenda step, so even the oldest caller hears about the roster first.
    assert.match(
      ((await headCreatedV1.json()) as { reason: string }).reason,
      /roster/,
    );
    const headCreatedV2 = await get('2026-10-01', 'b-head');
    assert.equal(headCreatedV2.status, 422);
    assert.match(
      ((await headCreatedV2.json()) as { reason: string }).reason,
      /roster/,
    );

    // An ambiguous text block is refused, not guessed: a title containing the
    // separator makes the old encoding undecodable, so the upcast refuses
    // before any handler runs, and the refusal stores nothing at all.
    const ambiguous = await post(
      '2026-06-01',
      { agenda: 'A;B;45', id: 'b-amb', roster: [], title: 'Sync' },
      'ambiguous',
    );
    assert.equal(ambiguous.status, 400);
    assert.equal(state.writes, 3, 'the refused write never committed');
    assert.equal(state.rows.has('b-amb'), false, 'refused facts never land');
    assert.equal((await get('2026-06-01', 'b-amb')).status, 404);
    // Extra separators and signed durations are equally undecodable, in every
    // language, because the encoding has exactly one shape.
    for (const agenda of ['Kickoff;60;extra', 'Kickoff;-5', 'Kickoff;+5']) {
      const refused = await post(
        '2026-06-01',
        { agenda, id: 'b-shape', roster: [], title: 'Sync' },
        'shape',
      );
      assert.equal(refused.status, 400, agenda);
    }
    assert.equal(state.writes, 3);

    // The behaviour switch gates writes, not history: a blank title accepted
    // by the oldest release stays readable at Head until the product decides
    // otherwise.
    const blankAtHead = await get(undefined, 'b-blank');
    assert.equal(blankAtHead.status, 200);
    assert.equal((await blankAtHead.json()).title, '   ');

    // Two keys for one id are two commands: both succeed and the later write
    // wins. Uniqueness of ids is the application's rule, not the chain's, and
    // the rewrite never erases the facts the oldest release still reads.
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
    const dupeV1View = await (await get('2026-06-01', 'b-dupe')).json();
    assert.equal(
      dupeV1View.agenda,
      'First;10',
      'storage only moves forward: the rewrite kept the oldest facts',
    );
    assert.deepEqual(dupeV1View.roster, []);
    const headRead = await get(undefined, 'b1');
    const headView = await headRead.json();
    assert.equal(headView.agenda, undefined);
    assert.equal(headView.organizer, 'user-9');

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
