import { createServer } from 'node:http';
import {
  hooks,
  resolveVersion,
  rfc3339ToEpoch,
  VERSION_IDS,
} from './transforms_hooks';
import type * as types from './versioned_transforms';
import * as a from './versioned_transforms.adapters';
import * as errors from './versioned_transforms.errors';

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
  /** Profiles the application sealed: Head raises the refinement for them. */
  sealed: Set<string>;
  writes: number;
}

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
  const parsed = rfc3339ToEpoch(createdAtUtc ?? '');
  state.rows.set(id, {
    createdAt:
      typeof raw.created_at === 'number'
        ? raw.created_at
        : parsed === undefined
          ? previous?.createdAt
          : parsed,
    firstName:
      typeof raw.first_name === 'string' ? raw.first_name : previous?.firstName,
    lastName:
      typeof raw.last_name === 'string' ? raw.last_name : previous?.lastName,
    note:
      (raw.note as { text: string } | undefined) ??
      (notes !== undefined
        ? // A Head caller owns the whole list: clearing it clears the fact.
          notes.length > 0
          ? notes[0]
          : undefined
        : undefined) ??
      (notes === undefined ? previous?.note : undefined),
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
    // E5 downward: the read time on the Head instance is the newest fact, so
    // the retired value derives from it before storage is consulted.
    profileStatus: source =>
      source.read_at !== undefined ? 'read' : state.rows.get(source.id)?.status,
  };
  return createServer(async (request, response) => {
    try {
      const url = new URL(request.url ?? '/', 'http://localhost');
      if (url.pathname === '/roster') {
        await serveRoster(request, response, state, stored, hooks);
        return;
      }
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
            if (state.sealed.has(profileRequest.id)) {
              return {
                error: new errors.Sealed({ id: profileRequest.id }),
                ok: false as const,
                stored,
              };
            }
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
          // Declared errors carry their status; downgraded refinements keep
          // the committed status on the runtime channel.
          const failure = result.value.error as {
            kind?: string;
            status?: number;
            error?: unknown;
          };
          const runtime =
            failure.kind === 'Framework'
              ? (failure.error as { status?: number; reason?: string })
              : undefined;
          const status = runtime?.status ?? failure.status ?? 404;
          response.setHeader('content-type', 'application/json');
          response
            .writeHead(status)
            .end(JSON.stringify({ reason: runtime?.reason ?? failure.kind }));
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

/** E9: the whole roster in one read; every element projects with its own facts. */
async function serveRoster(
  request: import('node:http').IncomingMessage,
  response: import('node:http').ServerResponse,
  state: DirectoryState,
  stored: a.StoredValues,
  hooks: a.VersionAdapters,
): Promise<void> {
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
  const result = await a.dispatchDirectoryRoster(
    version,
    {},
    async () => {
      const all = [...state.ledger.values()];
      if (all.length === 0) {
        return {
          error: { error: 'no profiles yet', kind: 'Framework' as const },
          ok: false as const,
          stored,
        };
      }
      return { ok: true as const, stored, value: all };
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
}
