import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import type * as types from './versioned_chain';
import * as a from './versioned_chain.adapters';

const SERVED_DATES = ['2027-01-01', '2026-10-01', '2026-06-01'] as const;
const VERSION_IDS = {
  '2026-06-01': 'v2026_06_01',
  '2026-10-01': 'v2026_10_01',
  '2027-01-01': 'v2027_01_01',
} as const;
type VersionId = (typeof VERSION_IDS)[keyof typeof VERSION_IDS];

/**
 * Runtime base one: the application resolves the version header. An absent
 * header serves Head; a date resolves to the newest served release not later
 * than it; anything earlier or malformed is a 400, never a guess.
 */
const resolveVersion = (header: string | undefined): VersionId | undefined => {
  if (header === undefined) return VERSION_IDS['2027-01-01'];
  if (!/^\d{4}-\d{2}-\d{2}$/.test(header)) return undefined;
  const served = SERVED_DATES.find(date => date <= header);
  return served === undefined ? undefined : VERSION_IDS[served];
};

/** Runtime base four: behaviour changes are named switches with a start date. */
const BEHAVIORS = { rejectBlankTitle: { since: '2027-01-01' } } as const;
const isOn = (behavior: keyof typeof BEHAVIORS, servedDate: string): boolean =>
  servedDate >= BEHAVIORS[behavior].since;

const hooks: a.VersionAdapters = {
  bookingDowncastToV20260601: (source, storedValues) => {
    const agenda = storedValues.bookingAgenda(source);
    return agenda === undefined
      ? {
          error: {
            direction: 'downcast' as const,
            reason: 'no stored agenda',
            typePath: 'Booking.agenda',
            version: '2026-06-01',
          },
          ok: false,
        }
      : { ok: true, value: { agenda } };
  },
  bookingDowncastToV20261001: (source, storedValues) => {
    const roster = storedValues.bookingRoster(source);
    return roster === undefined
      ? {
          error: {
            direction: 'downcast' as const,
            reason: 'no stored roster',
            typePath: 'Booking.roster',
            version: '2026-10-01',
          },
          ok: false,
        }
      : { ok: true, value: { roster } };
  },
  bookingUpcastFromV20260601: source => {
    const [title, minutes] = source.agenda.split(';');
    return {
      ok: true,
      value: {
        agenda: a.IGNORED,
        items: [{ minutes: Number(minutes), title }],
        locked: false,
      },
    };
  },
  bookingUpcastFromV20261001: () => ({
    ok: true,
    value: { roster: a.IGNORED },
  }),
};

export async function runChainHttpJourney(): Promise<void> {
  // Storage only moves forward: one row per booking, new facts appended.
  const rows = new Map<string, { agenda: string; roster: string[] }>();
  const stored: a.StoredValues = {
    bookingAgenda: source => rows.get(source.id)?.agenda,
    bookingRoster: source => rows.get(source.id)?.roster,
  };
  // Fingerprints are taken on the upcast command: the same meaning sent from
  // any release produces the same fingerprint, so a retry is one booking.
  const ledger = new Map<
    string,
    { fingerprint: string; outcome: types.v2027_01_01.Booking }
  >();
  let writes = 0;

  const server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url ?? '/', 'http://localhost');
      if (url.pathname !== '/bookings') {
        response.writeHead(404).end();
        return;
      }
      const rawHeader = request.headers['x-booking-version'];
      const header = Array.isArray(rawHeader) ? rawHeader[0] : rawHeader;
      const version = resolveVersion(header);
      const resolvedDate = Object.entries(VERSION_IDS).find(
        ([, id]) => id === version,
      )?.[0];
      response.setHeader('x-booking-version', resolvedDate ?? 'unresolved');
      if (version === undefined) {
        response
          .writeHead(400)
          .end(JSON.stringify({ reason: 'unsupported X-Booking-Version' }));
        return;
      }

      if (request.method === 'GET') {
        const id = url.searchParams.get('id') ?? '';
        const result = await a.dispatchSchedulerGet(
          version,
          { id },
          async (_request, context) => {
            assert.equal(context.originalVersionId, version);
            const head = ledger.get(`head:${id}`);
            if (head === undefined) {
              return {
                error: { error: 'no such booking', kind: 'Framework' as const },
                ok: false as const,
                stored,
              };
            }
            return { ok: true as const, stored, value: head.outcome };
          },
          hooks,
        );
        response.setHeader('content-type', 'application/json');
        if (!result.dispatched) {
          response.writeHead(400).end();
          return;
        }
        if (!result.projected) {
          response.writeHead(422).end();
          return;
        }
        if (!result.value.ok) {
          response.writeHead(404).end();
          return;
        }
        response.writeHead(200).end(JSON.stringify(result.value.value));
        return;
      }
      if (request.method !== 'POST') {
        response.writeHead(404).end();
        return;
      }

      const chunks: Uint8Array[] = [];
      for await (const chunk of request) chunks.push(chunk);
      const raw: unknown = JSON.parse(Buffer.concat(chunks).toString());
      const key = String(request.headers['idempotency-key']);
      const result = await a.dispatchSchedulerBook(
        version,
        { booking: raw },
        async (input, _context) => {
          // The behaviour switch fires on the resolved version, not the caller's.
          if (
            isOn('rejectBlankTitle', resolvedDate ?? '') &&
            input.booking.title.trim() === ''
          ) {
            return {
              error: { error: 'blank title', kind: 'Framework' as const },
              ok: false as const,
              stored,
            };
          }
          const fingerprint = JSON.stringify(input);
          const cached = ledger.get(key);
          if (cached) {
            return cached.fingerprint === fingerprint
              ? { ok: true as const, stored, value: cached.outcome }
              : {
                  error: {
                    error: 'idempotency key conflict',
                    kind: 'Framework' as const,
                  },
                  ok: false as const,
                  stored,
                };
          }
          writes++;
          ledger.set(key, { fingerprint, outcome: input.booking });
          ledger.set(`head:${input.booking.id}`, {
            fingerprint,
            outcome: input.booking,
          });
          // Storage appends what older releases will still read. The create
          // command acknowledged the retired roster (`xidl.ignored`) on its
          // way into Head, so the row keeps what the application wrote.
          rows.set(input.booking.id, {
            agenda: `${input.booking.items[0]?.title ?? ''};${input.booking.items[0]?.minutes ?? 0}`,
            roster: [],
          });
          return { ok: true as const, stored, value: input.booking };
        },
        hooks,
      );
      response.setHeader('content-type', 'application/json');
      if (!result.dispatched) {
        response.writeHead(400).end(JSON.stringify(result.error));
        return;
      }
      if (!result.projected) {
        response
          .writeHead(422)
          .end(JSON.stringify({ reason: result.error.reason }));
        return;
      }
      if (!result.value.ok) {
        const framework =
          (result.value.error as { kind?: string }).kind === 'Framework';
        const reason = framework
          ? String((result.value.error as { error: unknown }).error)
          : 'refused';
        response
          .writeHead(reason.includes('conflict') ? 409 : 422)
          .end(JSON.stringify({ reason }));
        return;
      }
      response.writeHead(200).end(JSON.stringify(result.value.value));
    } catch (error) {
      response.writeHead(500).end(String(error));
    }
  });
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
    assert.equal(writes, 1);
    // Head reads the same booking: the stored agenda unpacked by the hook,
    // the implied locked value, and owner under its Head name.
    const asHead = (await (
      await get(undefined, 'b1')
    ).json()) as types.v2027_01_01.Booking;
    assert.deepEqual(asHead.items, [{ minutes: 60, title: 'Kickoff' }]);
    assert.equal(
      asHead.locked,
      false,
      'the upcast hook stated the implied value',
    );
    assert.equal(asHead.organizer, 'user-9');

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
        locked: false,
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
    assert.equal(writes, 1, 'the fingerprint lives on the upcast command');
    const conflict = await post(
      '2027-01-01',
      {
        id: 'b1',
        items: [{ minutes: 30, title: 'Changed' }],
        locked: false,
        organizer: 'user-9',
        title: 'Changed',
      },
      'kickoff',
    );
    assert.equal(conflict.status, 409, 'same key, different command');
    assert.equal(writes, 1);

    // Behaviour switch: Head refuses a blank title; the oldest release never
    // had that behaviour and accepts the same request.
    const blankV1 = { agenda: ' ;0', id: 'b-blank', roster: [], title: '   ' };
    assert.equal((await post('2026-06-01', blankV1, 'blank-v1')).status, 200);
    const blankHead = await post(
      undefined,
      {
        id: 'b-blank-head',
        items: [{ minutes: 0, title: '   ' }],
        locked: false,
        title: '   ',
      },
      'blank-head',
    );
    assert.equal(blankHead.status, 422);
    assert.equal(writes, 2, 'only the accepted bookings were written');

    // Old releases read the booking back with what storage actually holds.
    // The create command acknowledged the roster (`xidl.ignored`) when it
    // crossed into Head, so the row keeps the empty roster it was written
    // with; the conversion serves that fact and never invents the caller's.
    const v1Read = await get('2026-06-01', 'b1');
    assert.equal(v1Read.status, 200);
    const v1View = await v1Read.json();
    assert.equal(v1View.agenda, 'Kickoff;60');
    assert.deepEqual(v1View.roster, []);
    assert.equal(v1View.owner, 'user-9');
    assert.equal(Object.hasOwn(v1View, 'items'), false);
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
