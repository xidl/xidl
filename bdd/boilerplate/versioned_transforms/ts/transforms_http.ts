import assert from 'node:assert/strict';
import type { AddressInfo } from 'node:net';
import { createDirectoryApp, type DirectoryState } from './transforms_app';
import type * as types from './versioned_transforms';

export async function runTransformsHttpJourney(): Promise<void> {
  const state: DirectoryState = {
    ledger: new Map(),
    rows: new Map(),
    sealed: new Set(),
    writes: 0,
  };
  const server = createDirectoryApp(state);
  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
  try {
    const base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
    const post = (version: string | undefined, body: unknown) =>
      fetch(`${base}/profiles`, {
        body: JSON.stringify(body),
        headers: {
          'content-type': 'application/json',
          ...(version === undefined ? {} : { 'x-transform-version': version }),
        },
        method: 'POST',
      });
    const get = (version: string | undefined, id: string) =>
      fetch(`${base}/profiles?id=${id}`, {
        headers:
          version === undefined ? {} : { 'x-transform-version': version },
      });
    const roster = (version: string | undefined) =>
      fetch(`${base}/roster`, {
        headers:
          version === undefined ? {} : { 'x-transform-version': version },
      });

    // Negotiation: an impossible calendar date is a 400, never a guess.
    for (const header of ['2026-02-30', '2026-13-01', 'nope']) {
      const probe = await get(header, 'missing');
      assert.equal(probe.status, 400, header);
      assert.equal(probe.headers.get('x-transform-version'), 'unresolved');
    }

    // The oldest release publishes; the response is exactly its own contract.
    const oldest = {
      created_at: 1_700_000_000,
      first_name: 'Ann',
      id: 'p1',
      last_name: 'Lee',
      note: { text: 'prefers email' },
      recipient: 'anna@example.test',
      status: 'sent',
      title: 'founder',
    };
    const published = await post('2026-01-01', oldest);
    assert.equal(published.status, 200);
    assert.deepEqual(await published.json(), oldest);
    assert.equal(state.writes, 1);

    // Head reads the merged view: the mechanical display name, the stated
    // fallback (the old behaviour, never the new default), the structured
    // route without an invented protocol and the RFC 3339 creation time.
    const headView = (await (
      await get(undefined, 'p1')
    ).json()) as types.v2026_07_01.Profile;
    assert.equal(headView.display_name, 'Ann Lee');
    assert.equal(
      headView.fallback,
      true,
      'the upcast stated the old behaviour',
    );
    assert.deepEqual(headView.to, { handle: 'anna@example.test' });
    assert.equal(
      headView.tier,
      'standard',
      'the upcast states the tier; no default exists for it',
    );
    assert.equal(headView.created_at_utc, '2023-11-14T22:13:20.000Z');
    assert.deepEqual(headView.notes, [{ text: 'prefers email' }]);
    assert.equal(Object.hasOwn(headView, 'status'), false);

    // The middle release reads back its own contract, including the stated
    // fallback and the derivation-free stored facts.
    const v2View = (await (
      await get('2026-04-01', 'p1')
    ).json()) as types.v2026_04_01.Profile;
    assert.equal(v2View.created_at, 1_700_000_000);
    assert.equal(v2View.fallback, true);
    assert.equal(v2View.status, 'sent');
    assert.deepEqual(v2View.to, { handle: 'anna@example.test' });

    // Round-trip rule one: the oldest caller reads exactly what it wrote.
    assert.deepEqual(await (await get('2026-01-01', 'p1')).json(), oldest);

    // E5 upward over HTTP: a retired enum value is refused before the handler
    // runs, so the response is a bad request and nothing is committed.
    const refused = await post('2026-01-01', { ...oldest, status: 'read' });
    assert.equal(refused.status, 400);
    assert.equal(state.writes, 1);

    // The middle release publishes with the route it owns; the fallback it
    // does not send stays absent on the Head view, because a default is only
    // an implied value for callers who cannot state it yet.
    const announced = await post('2026-04-01', {
      created_at: 1_770_000_000,
      fallback: false,
      first_name: 'Ann',
      id: 'p2',
      last_name: 'Lee',
      note: { text: 'morning person' },
      status: 'delivered',
      title: 'engineer',
      to: { handle: 'lee@example.test', protocol: 'relay' },
    });
    assert.equal(announced.status, 200);
    assert.equal(state.writes, 2);
    const announcedHead = (await (
      await get(undefined, 'p2')
    ).json()) as types.v2026_07_01.Profile;
    assert.equal(announcedHead.display_name, 'Ann Lee');
    assert.equal(
      announcedHead.fallback,
      false,
      'the caller stated false; the default never touches it',
    );

    // The Head rewrite customises the display, grows the notes and switches
    // the route to the newest protocol value with a read time.
    const rewritten = await post(undefined, {
      created_at_utc: '2026-06-01T12:00:00.000Z',
      display_name: 'Dr. Ann Lee',
      id: 'p2',
      notes: [{ text: 'morning person' }, { text: 'vip' }],
      read_at: '2026-06-02T08:00:00.000Z',
      tier: 'premium',
      title: 'engineer',
      to: { handle: 'lee@example.test', protocol: 'chat' },
    });
    assert.equal(rewritten.status, 200);
    assert.equal(state.writes, 3);
    const rewrittenHead = (await (
      await get(undefined, 'p2')
    ).json()) as types.v2026_07_01.Profile;
    assert.equal(rewrittenHead.display_name, 'Dr. Ann Lee');
    // fallback is a live Head member: the rewriter sees it and owns it, so
    // omitting it clears it instead of resurrecting the stored value.
    assert.equal(rewrittenHead.fallback, undefined);

    // E4: the newest protocol value has no image in any release that still
    // declares the field. The chain stops at the first such release, so both
    // older callers are refused; the value only rides out with its field when
    // no release between declares it at all.
    for (const probe of ['2026-04-01', '2026-01-01']) {
      const chatRefusal = await get(probe, 'p2');
      assert.equal(chatRefusal.status, 422, probe);
      assert.match(
        ((await chatRefusal.json()) as { reason: string }).reason,
        /chat/,
      );
    }

    // E3 write rule: restating the stored name pair keeps the custom display.
    const restated = await post('2026-04-01', {
      created_at: 1_770_000_000,
      first_name: 'Ann',
      id: 'p2',
      last_name: 'Lee',
      status: 'delivered',
      title: 'engineer',
      to: { handle: 'lee@example.test', protocol: 'relay' },
    });
    assert.equal(restated.status, 200);
    const restatedHead = (await (
      await get(undefined, 'p2')
    ).json()) as types.v2026_07_01.Profile;
    assert.equal(
      restatedHead.display_name,
      'Dr. Ann Lee',
      'unchanged names never overwrite the custom display',
    );

    // E3 write rule: changing the names recomputes the mechanical merge.
    await post('2026-04-01', {
      created_at: 1_770_000_000,
      first_name: 'Anna',
      id: 'p2',
      last_name: 'Lee',
      status: 'delivered',
      title: 'engineer',
      to: { handle: 'lee@example.test', protocol: 'relay' },
    });
    assert.equal(
      ((await (await get(undefined, 'p2')).json()) as types.v2026_07_01.Profile)
        .display_name,
      'Anna Lee',
    );

    // E7 write rule: a single-note write keeps the entries it cannot see.
    const singleNote = await post('2026-04-01', {
      created_at: 1_770_000_000,
      first_name: 'Anna',
      id: 'p2',
      last_name: 'Lee',
      note: { text: 'call first' },
      status: 'delivered',
      title: 'engineer',
      to: { handle: 'lee@example.test', protocol: 'relay' },
    });
    assert.equal(singleNote.status, 200);
    assert.deepEqual(
      ((await (await get(undefined, 'p2')).json()) as types.v2026_07_01.Profile)
        .notes,
      [{ text: 'call first' }, { text: 'vip' }],
      'the invisible rest of the list survives the rewrite',
    );
    // Reading the middle release serves the visible head of the list.
    assert.deepEqual(
      (
        (await (
          await get('2026-04-01', 'p2')
        ).json()) as types.v2026_04_01.Profile
      ).note,
      { text: 'call first' },
    );

    // E10: the refinement raised at Head reaches the middle release through
    // the downgrade decision, keeping the committed status instead of being
    // translated into a missing profile.
    state.sealed.add('p1');
    const sealedV2 = await get('2026-04-01', 'p1');
    assert.equal(sealedV2.status, 409);
    assert.match(
      ((await sealedV2.json()) as { reason: string }).reason,
      /sealed/,
    );
    const sealedHead = await get(undefined, 'p1');
    assert.equal(sealedHead.status, 409);
    state.sealed.delete('p1');

    // E9: one roster read projects every element with its own facts.
    const rosterV2 = (await (
      await roster('2026-04-01')
    ).json()) as types.v2026_04_01.Profile[];
    assert.equal(rosterV2.length, 2);
    const rosterById = new Map(rosterV2.map(p => [p.id, p]));
    assert.equal(rosterById.get('p1')?.status, 'sent');
    assert.equal(rosterById.get('p1')?.note?.text, 'prefers email');
    assert.equal(rosterById.get('p2')?.note?.text, 'call first');
    assert.equal(rosterById.get('p2')?.created_at, 1_770_000_000);
    const rosterV1 = (await (
      await roster('2026-01-01')
    ).json()) as types.v2026_01_01.Profile[];
    assert.deepEqual(rosterV1.map(p => p.recipient).sort(), [
      'anna@example.test',
      'lee@example.test',
    ]);

    // A profile that only ever existed at Head cannot serve the pieces it
    // never had: the display name cannot be split, so the read refuses.
    await post(undefined, {
      created_at_utc: '2026-06-01T09:00:00.000Z',
      display_name: 'Solo',
      id: 'p3',
      notes: [],
      tier: 'standard',
      title: 'ghost',
      to: { handle: 'ghost@example.test' },
    });
    assert.equal((await get('2026-04-01', 'p3')).status, 422);

    // E7, the Head-clearing case: a caller that owns the list empties it, and
    // the note fact older callers read is gone with it, never a ghost.
    await post(undefined, {
      created_at_utc: '2026-06-01T12:00:00.000Z',
      display_name: 'Dr. Ann Lee',
      id: 'p2',
      notes: [],
      read_at: '2026-06-02T08:00:00.000Z',
      tier: 'premium',
      title: 'engineer',
      to: { handle: 'lee@example.test', protocol: 'relay' },
    });
    const clearedRead = await get('2026-04-01', 'p2');
    assert.equal(clearedRead.status, 200);
    const clearedView = (await clearedRead.json()) as Record<string, unknown>;
    assert.equal(Object.hasOwn(clearedView, 'note'), false);
    // E5 downward, end to end: the read time on the Head instance is the
    // newest fact, so the middle release serves the retired value from it.
    assert.equal(clearedView.status, 'read');

    // The creation stamp is parsed by one strict RFC 3339 shape in every
    // language: offsets and impossible calendar dates never become facts.
    for (const stamp of [
      '2026-06-01T12:00:00+02:00',
      '2026-02-31T00:00:00Z',
      '2026-06-01T25:00:00Z',
    ]) {
      await post(undefined, {
        created_at_utc: stamp,
        display_name: 'Stamped',
        id: 'p-stamp',
        notes: [],
        tier: 'standard',
        title: 'stamp',
        to: { handle: 'stamp@example.test' },
      });
      const stampRead = await get('2026-04-01', 'p-stamp');
      assert.equal(stampRead.status, 422, stamp);
      assert.match(
        ((await stampRead.json()) as { reason: string }).reason,
        /creation time/,
      );
    }

    // One element without its stored facts refuses the whole read: the list
    // cannot be served partially without inventing the missing pieces.
    await post(undefined, {
      created_at_utc: '2026-06-01T10:00:00.000Z',
      display_name: 'Head Born',
      id: 'p5',
      notes: [],
      tier: 'premium',
      title: 'newest',
      to: { handle: 'new@example.test' },
    });
    const brokenRoster = await roster('2026-04-01');
    assert.equal(brokenRoster.status, 422);
    assert.match(
      ((await brokenRoster.json()) as { reason: string }).reason,
      /display name/,
    );

    console.log(
      'PASS: transform shapes negotiate, project, refuse and round-trip over HTTP',
    );
  } finally {
    server.closeAllConnections();
    await new Promise<void>((resolve, fail) =>
      server.close(error => (error ? fail(error) : resolve())),
    );
  }
}
