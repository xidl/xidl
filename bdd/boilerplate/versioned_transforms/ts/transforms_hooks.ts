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

const ok = <T>(value: T): a.VersionedResult<T> => ({ ok: true, value });

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
  // E10: the oldest contracts cannot name the refinement, so the downgrade
  // keeps the committed status on the runtime channel instead of translating
  // a sealed profile into a missing one.
  directoryProfileSealedErrorDowncastToV20260401: source =>
    ok({
      error: { reason: `profile ${source.id} is sealed`, status: 409 },
      kind: 'Framework',
    }),
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
        // E12: a required member arrives at Head; the decision states the
        // application's rule for callers who cannot name the member yet.
        recipient: a.IGNORED,
        tier: 'standard',
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
        tier: 'standard',
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
