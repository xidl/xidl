import assert from 'node:assert/strict';
import test from 'node:test';
import { z } from 'zod';
import { createAdaptedRouter, defineOperation } from '../src/index.js';

const responseSpec = {
  bodyFields: [{ key: 'return', wireName: 'return' }],
  bodyMode: 'return' as const,
  contentType: 'application/json',
  cookies: [],
  headers: [],
  status: 200,
  stream: false,
};

test('createAdaptedRouter preserves raw wire body keys while coercing path scalars', async () => {
  interface UpdateService {
    update(id: number, body: { fullName: string }): { id: number; ok: boolean };
  }

  const updateBodySchema = z.object({
    fullName: z.string().meta({ xjson: { rename: 'full_name' } }),
  });

  const operations = {
    update: defineOperation<UpdateService, 'update'>({
      handler: 'update',
      method: 'POST',
      path: '/items/{id}',
      request: {
        args: ['id', 'body'],
        body: {
          contentType: 'application/json',
          fields: [{ key: 'body', wireName: 'body' }],
          kind: 'value',
          schema: updateBodySchema,
          singleKey: 'body',
        },
        cookies: [],
        headers: [],
        kind: 'object',
        path: [
          {
            catchAll: false,
            key: 'id',
            multi: false,
            parameter: 'id',
            wireName: 'id',
          },
        ],
        query: [],
        schema: z.object({
          body: updateBodySchema,
          id: z.coerce.number().int(),
        }),
      },
      response: responseSpec,
      security: [],
    }),
  } as const;

  let seenRaw: Record<string, unknown> | undefined;
  const issue = { message: 'field is not available in served version' };
  const router = createAdaptedRouter('v1', operations, {
    update: {
      dispatch: (version, raw) => {
        seenRaw = raw;
        const body = raw.body as Record<string, unknown>;
        if ('future_field' in body) {
          return {
            dispatched: false,
            error: {
              issues: [issue],
              kind: 'Validation',
              operation: 'Items.update',
              version,
            },
          };
        }
        return {
          dispatched: true,
          projected: true,
          value: {
            ok: true,
            value: { id: raw.id, ok: body.full_name === 'Ada' },
          },
        };
      },
    },
  });

  const okRes = await router(
    new Request('http://localhost/items/42', {
      body: JSON.stringify({ full_name: 'Ada' }),
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
    }),
  );
  assert.equal(okRes.status, 200);
  assert.deepEqual(await okRes.json(), { id: 42, ok: true });
  assert.deepEqual(seenRaw, { body: { full_name: 'Ada' }, id: 42 });

  const rejectedRes = await router(
    new Request('http://localhost/items/42', {
      body: JSON.stringify({ full_name: 'Ada', future_field: 'v2-only' }),
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
    }),
  );
  assert.equal(rejectedRes.status, 400);
  assert.deepEqual(await rejectedRes.json(), {
    code: 400,
    detail: [issue],
    msg: 'request validation failed',
  });
});
