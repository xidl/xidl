import { createServer } from 'node:http';
import type * as types from './versioned_chain';
import * as a from './versioned_chain.adapters';

export const SERVED_DATES = ['2027-01-01', '2026-10-01', '2026-06-01'] as const;
export const VERSION_IDS = {
  '2026-06-01': 'v2026_06_01',
  '2026-10-01': 'v2026_10_01',
  '2027-01-01': 'v2027_01_01',
} as const;
export type VersionId = (typeof VERSION_IDS)[keyof typeof VERSION_IDS];

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

/** A calendar-real date, not merely a matching shape: 2026-02-30 is not one. */
const isCalendarDate = (value: string): boolean => {
  if (!ISO_DATE.test(value)) return false;
  const parsed = Date.parse(`${value}T00:00:00Z`);
  return (
    !Number.isNaN(parsed) &&
    new Date(parsed).toISOString().slice(0, 10) === value
  );
};

/**
 * Runtime base one: the application resolves the version header. An absent
 * header serves Head; a date resolves to the newest served release not later
 * than it; anything earlier, malformed or calendar-impossible is a 400, never
 * a guess.
 */
export const resolveVersion = (
  header: string | undefined,
): VersionId | undefined => {
  if (header === undefined) return VERSION_IDS['2027-01-01'];
  if (!isCalendarDate(header)) return undefined;
  const served = SERVED_DATES.find(date => date <= header);
  return served === undefined ? undefined : VERSION_IDS[served];
};

/** Runtime base four: behaviour changes are named switches with a start date. */
const BEHAVIORS = { rejectBlankTitle: { since: '2027-01-01' } } as const;
const isOn = (behavior: keyof typeof BEHAVIORS, servedDate: string): boolean =>
  servedDate >= BEHAVIORS[behavior].since;

/** Pure conversion decisions; hooks never touch storage. */
export const hooks: a.VersionAdapters = {
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
    // One separator only: the rest after the first ';' is the whole duration,
    // so 'A;B;45' and 'Kickoff;60;extra' are undecodable, never guessed.
    const separator = source.agenda.indexOf(';');
    const title = source.agenda.slice(0, Math.max(separator, 0));
    const minutes =
      separator === -1 ? undefined : source.agenda.slice(separator + 1);
    // A lossy or ambiguous text block is a refusal, never a silent guess.
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
    return {
      ok: true,
      value: {
        agenda: a.IGNORED,
        items: [{ minutes: duration, title }],
        locked: false,
      },
    };
  },
  bookingUpcastFromV20261001: () => ({
    ok: true,
    value: { roster: a.IGNORED },
  }),
};

export interface BookingAppState {
  /** Storage only moves forward: one row per booking, facts merged in. */
  rows: Map<
    string,
    { agenda: string | undefined; roster: string[] | undefined }
  >;
  /** Fingerprints live on the upcast command: same meaning, same fingerprint. */
  ledger: Map<
    string,
    { fingerprint: string; outcome: types.v2027_01_01.Booking }
  >;
  writes: number;
}

/** The booking application server: routing, storage, idempotency, behaviours. */
export function createBookingApp(state: BookingAppState) {
  const stored: a.StoredValues = {
    bookingAgenda: source => state.rows.get(source.id)?.agenda,
    bookingRoster: source => state.rows.get(source.id)?.roster,
  };
  return createServer(async (request, response) => {
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
            if (context.originalVersionId !== version) {
              throw new Error(
                'the dispatcher must report the resolved version',
              );
            }
            const head = state.ledger.get(`head:${id}`);
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
      let raw: unknown;
      try {
        raw = JSON.parse(Buffer.concat(chunks).toString());
      } catch {
        response
          .writeHead(400)
          .end(JSON.stringify({ reason: 'malformed JSON body' }));
        return;
      }
      const keyHeader = request.headers['idempotency-key'];
      if (keyHeader === undefined) {
        response
          .writeHead(400)
          .end(
            JSON.stringify({ reason: 'idempotency-key header is required' }),
          );
        return;
      }
      const key = String(keyHeader);
      // The application reads its own wire: facts older releases still read go
      // to storage straight from the raw request, and only ever move forward —
      // a release whose contract dropped a fact leaves the stored one alone.
      // The facts land when the command commits; refused requests change
      // nothing at all.
      const rememberFacts = () => {
        const booking = raw as {
          id?: unknown;
          agenda?: unknown;
          roster?: unknown;
        };
        const id = String(booking.id ?? '');
        const previous = state.rows.get(id);
        state.rows.set(id, {
          agenda:
            booking.agenda !== undefined && typeof booking.agenda === 'string'
              ? booking.agenda
              : previous?.agenda,
          roster:
            booking.roster !== undefined && Array.isArray(booking.roster)
              ? booking.roster.map(String)
              : previous?.roster,
        });
      };
      const result = await a.dispatchSchedulerBook(
        version,
        { booking: raw },
        async (input, _context) => {
          // A replay answers before any behaviour gate: the original decision
          // stands regardless of when the retry arrives.
          const fingerprint = JSON.stringify(input);
          const cached = state.ledger.get(key);
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
          state.writes++;
          rememberFacts();
          state.ledger.set(key, { fingerprint, outcome: input.booking });
          state.ledger.set(`head:${input.booking.id}`, {
            fingerprint,
            outcome: input.booking,
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
          .writeHead(reason === 'idempotency key conflict' ? 409 : 422)
          .end(JSON.stringify({ reason }));
        return;
      }
      response.writeHead(200).end(JSON.stringify(result.value.value));
    } catch (error) {
      response.writeHead(500).end(String(error));
    }
  });
}
