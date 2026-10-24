import { createServer } from 'node:http';
import type * as types from './versioned_transforms';
import * as a from './versioned_transforms.adapters';

export const SERVED_DATES = ['2026-07-01', '2026-04-01', '2026-01-01'] as const;
export const VERSION_IDS = {
  '2026-01-01': 'v2026_01_01',
  '2026-04-01': 'v2026_04_01',
  '2026-07-01': 'v2026_07_01',
} as const;
export type VersionId = (typeof VERSION_IDS)[keyof typeof VERSION_IDS];

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

const isCalendarDate = (value: string): boolean => {
  if (!ISO_DATE.test(value)) return false;
  const parsed = Date.parse(`${value}T00:00:00Z`);
  return (
    !Number.isNaN(parsed) &&
    new Date(parsed).toISOString().slice(0, 10) === value
  );
};

/** The application resolves the header; an impossible date is never a guess. */
export const resolveVersion = (
  header: string | undefined,
): VersionId | undefined => {
  if (header === undefined) return VERSION_IDS['2026-07-01'];
  if (!isCalendarDate(header)) return undefined;
  const served = SERVED_DATES.find(date => date <= header);
  return served === undefined ? undefined : VERSION_IDS[served];
};

/** What older releases still read, keyed by profile id. */
export interface ProfileFacts {
  createdAt: number | undefined;
  firstName: string | undefined;
  lastName: string | undefined;
  note: { text: string } | undefined;
  status: string | undefined;
  recipient: string | undefined;
}

export interface DirectoryState {
  rows: Map<string, ProfileFacts>;
  ledger: Map<string, types.v2026_07_01.Profile>;
  writes: number;
}

const refuse = (
  typePath: string,
  reason: string,
  version: string,
): a.VersionedResult<never> => ({
  error: { direction: 'upcast', reason, typePath, version },
  ok: false,
});

/**
 * Pure conversion decisions. Facts retired from the Head contract come only
 * from storage; values the Head instance still carries may be derived, and
 * every derivation is declared lossy or refusing, never inventive.
 */
export const hooks: a.VersionAdapters = {
  profileDowncastToV20260101: (source, stored) => {
    const recipient = stored.profileRecipient(source);
    return recipient === undefined
      ? {
          error: {
            direction: 'downcast',
            reason: `no stored recipient for profile ${source.id}`,
            typePath: 'Profile.recipient',
            version: '2026-01-01',
          },
          ok: false,
        }
      : { ok: true, value: { recipient } };
  },
  profileDowncastToV20260401: (source, stored) => {
    const createdAt = stored.profileCreatedAt(source);
    if (createdAt === undefined) {
      return {
        error: {
          direction: 'downcast',
          reason: `no stored creation time for profile ${source.id}`,
          typePath: 'Profile.created_at',
          version: '2026-04-01',
        },
        ok: false,
      };
    }
    const firstName = stored.profileFirstName(source);
    if (firstName === undefined) {
      return {
        error: {
          direction: 'downcast',
          reason: `the display name of profile ${source.id} cannot be split`,
          typePath: 'Profile.first_name',
          version: '2026-04-01',
        },
        ok: false,
      };
    }
    const lastName = stored.profileLastName(source);
    if (lastName === undefined) {
      return {
        error: {
          direction: 'downcast',
          reason: `the display name of profile ${source.id} cannot be split`,
          typePath: 'Profile.last_name',
          version: '2026-04-01',
        },
        ok: false,
      };
    }
    const status = stored.profileStatus(source);
    if (status === undefined) {
      return {
        error: {
          direction: 'downcast',
          reason: `no stored status for profile ${source.id}`,
          typePath: 'Profile.status',
          version: '2026-04-01',
        },
        ok: false,
      };
    }
    const note = stored.profileNote(source);
    return {
      ok: true,
      value: {
        created_at: createdAt,
        first_name: firstName,
        last_name: lastName,
        note: note === undefined ? a.OMIT : note,
        status,
      },
    };
  },
  // E8 inverted: older callers' behaviour matched true while the new member
  // defaults to false, so the upcast states true and never the default.
  profileUpcastFromV20260101: source => {
    // E5 upward: "read" retired into a timestamp nobody can invent.
    if (source.status === 'read') {
      return refuse(
        'Profile.read_at',
        `a read time for profile ${source.id} can never be invented`,
        '2026-01-01',
      );
    }
    return {
      ok: true,
      value: {
        fallback: true,
        recipient: a.IGNORED,
        to: { handle: source.recipient },
      },
    };
  },
  profileUpcastFromV20260401: source => {
    if (source.status === 'read') {
      return refuse(
        'Profile.read_at',
        `a read time for profile ${source.id} can never be invented`,
        '2026-04-01',
      );
    }
    return {
      ok: true,
      value: {
        created_at: a.IGNORED,
        // E6 upward: seconds to RFC 3339, deterministic and lossless.
        created_at_utc: new Date(source.created_at * 1000).toISOString(),
        // E3: the merge is mechanical; whether a custom display survives a
        // rewrite is the application's rule, not the chain's.
        display_name: `${source.first_name} ${source.last_name}`,
        first_name: a.IGNORED,
        last_name: a.IGNORED,
        note: a.IGNORED,
        // E7 upward: one optional note becomes a one-entry list.
        notes: source.note === undefined ? [] : [source.note],
        read_at: undefined,
        status: a.IGNORED,
      },
    };
  },
  // E4: the newest value simply has no image one release earlier.
  routeProtocolDowncastToV20260401: source => ({
    error: {
      direction: 'downcast',
      reason: `${source} has no image in the 2026-04-01 contract`,
      typePath: 'Route.protocol',
      version: '2026-04-01',
    },
    ok: false,
  }),
};

/** The application stores its own wire facts; only committed facts land. */
export const rememberFacts = (
  state: DirectoryState,
  id: string,
  raw: Record<string, unknown>,
): void => {
  const previous = state.rows.get(id);
  const notes = raw.notes as Array<{ text: string }> | undefined;
  const to = raw.to as { handle?: unknown } | undefined;
  const createdAtUtc = raw.created_at_utc as string | undefined;
  const parsed =
    createdAtUtc === undefined ? Number.NaN : Date.parse(createdAtUtc);
  state.rows.set(id, {
    createdAt:
      typeof raw.created_at === 'number'
        ? raw.created_at
        : Number.isNaN(parsed)
          ? previous?.createdAt
          : Math.floor(parsed / 1000),
    firstName:
      typeof raw.first_name === 'string' ? raw.first_name : previous?.firstName,
    lastName:
      typeof raw.last_name === 'string' ? raw.last_name : previous?.lastName,
    note:
      (raw.note as { text: string } | undefined) ??
      (notes !== undefined && notes.length > 0 ? notes[0] : undefined) ??
      previous?.note,
    recipient:
      typeof raw.recipient === 'string'
        ? raw.recipient
        : to !== undefined && typeof to.handle === 'string'
          ? to.handle
          : previous?.recipient,
    status: typeof raw.status === 'string' ? raw.status : previous?.status,
  });
};

/** The directory application server: routing, storage, refusals. */
export function createDirectoryApp(state: DirectoryState) {
  const stored: a.StoredValues = {
    profileCreatedAt: source => state.rows.get(source.id)?.createdAt,
    profileFirstName: source => state.rows.get(source.id)?.firstName,
    profileLastName: source => state.rows.get(source.id)?.lastName,
    profileNote: source => state.rows.get(source.id)?.note,
    profileRecipient: source => state.rows.get(source.id)?.recipient,
    profileStatus: source => state.rows.get(source.id)?.status,
  };
  return createServer(async (request, response) => {
    try {
      const url = new URL(request.url ?? '/', 'http://localhost');
      if (url.pathname !== '/profiles') {
        response.writeHead(404).end();
        return;
      }
      const rawHeader = request.headers['x-transform-version'];
      const header = Array.isArray(rawHeader) ? rawHeader[0] : rawHeader;
      const version = resolveVersion(header);
      const resolvedDate = Object.entries(VERSION_IDS).find(
        ([, id]) => id === version,
      )?.[0];
      response.setHeader('x-transform-version', resolvedDate ?? 'unresolved');
      if (version === undefined) {
        response.writeHead(400).end();
        return;
      }
      if (request.method === 'GET') {
        const id = url.searchParams.get('id') ?? '';
        const result = await a.dispatchDirectoryProfile(
          version,
          { id },
          async profileRequest => {
            const head = state.ledger.get(profileRequest.id);
            if (head === undefined) {
              return {
                error: { error: 'no such profile', kind: 'Framework' as const },
                ok: false as const,
                stored,
              };
            }
            return { ok: true as const, stored, value: head };
          },
          hooks,
        );
        response.setHeader('content-type', 'application/json');
        if (!result.dispatched) {
          response.writeHead(400).end();
          return;
        }
        if (!result.projected) {
          response
            .writeHead(422)
            .end(JSON.stringify({ reason: result.error?.reason }));
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
        response.writeHead(400).end();
        return;
      }
      const result = await a.dispatchDirectoryPublish(
        version,
        { profile: raw },
        async (input, context) => {
          const id = input.profile.id;
          const previous = state.ledger.get(id);
          const facts = state.rows.get(id);
          const oldCaller = context.originalVersionId !== 'v2026_07_01';
          // E3 write rule: a caller restating the stored name pair keeps the
          // custom display; a caller changing the names gets the mechanical
          // merge the hook computed.
          if (
            previous !== undefined &&
            facts?.firstName !== undefined &&
            (raw as { first_name?: unknown }).first_name === facts.firstName &&
            (raw as { last_name?: unknown }).last_name === facts.lastName
          ) {
            input.profile.display_name = previous.display_name;
          }
          // E7 write rule: only older callers write through the narrow view.
          // A stated note replaces the visible head of the list and keeps the
          // entries it cannot see; an omitted note changes nothing.
          const mergedNotes = !oldCaller
            ? input.profile.notes
            : input.profile.notes.length === 0
              ? (previous?.notes ?? [])
              : [input.profile.notes[0], ...(previous?.notes.slice(1) ?? [])];
          input.profile = { ...input.profile, notes: mergedNotes };
          rememberFacts(state, id, raw as Record<string, unknown>);
          state.writes++;
          state.ledger.set(id, input.profile);
          return { ok: true as const, stored, value: input.profile };
        },
        hooks,
      );
      response.setHeader('content-type', 'application/json');
      if (!result.dispatched) {
        response.writeHead(400).end();
        return;
      }
      if (!result.projected) {
        response
          .writeHead(422)
          .end(JSON.stringify({ reason: result.error?.reason }));
        return;
      }
      if (!result.value.ok) {
        response.writeHead(500).end();
        return;
      }
      response.writeHead(200).end(JSON.stringify(result.value.value));
    } catch {
      response.writeHead(500).end();
    }
  });
}
