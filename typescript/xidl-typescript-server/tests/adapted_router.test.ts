import assert from 'node:assert/strict';
import test from 'node:test';
import { z } from 'zod';

import {
  createAdaptedRouter,
  createVersionedAdapterRouter,
  defineOperation,
  XidlServerError,
} from '../src/index.js';

class V1NotFound extends Error {
  readonly id: string;
  constructor(data: { id: string }) {
    super(`not found: ${data.id}`);
    this.id = data.id;
  }
}

class V2NotFound extends Error {
  readonly id: string;
  readonly reason: string;
  constructor(data: { id: string; reason: string }) {
    super(`not found: ${data.id}`);
    this.id = data.id;
    this.reason = data.reason;
  }
}

interface V1Service {
  getItem(id: string): { id: string; label: string };
}

interface V2Service {
  getItem(id: string): { id: string; title: string };
}

const requestSpec = {
  args: ['id'],
  body: { contentType: '', fields: [], kind: 'none' as const },
  cookies: [],
  headers: [],
  kind: 'object' as const,
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
  schema: z.object({ id: z.string() }),
};

const responseSpec = {
  bodyFields: [{ key: 'return', wireName: 'return' }],
  bodyMode: 'return' as const,
  contentType: 'application/json',
  cookies: [],
  headers: [],
  status: 200,
  stream: false,
};

const v1Operations = {
  getItem: defineOperation<V1Service, 'getItem'>({
    errors: {
      match: error =>
        error instanceof V1NotFound
          ? {
              body: { id: error.id },
              headers: new Headers({ 'Content-Type': 'application/json' }),
              status: 404,
            }
          : undefined,
    },
    handler: 'getItem',
    method: 'GET',
    path: '/items/{id}',
    request: requestSpec,
    response: responseSpec,
    security: [],
  }),
} as const;

const v2Operations = {
  getItem: defineOperation<V2Service, 'getItem'>({
    errors: {
      match: error =>
        error instanceof V2NotFound
          ? {
              body: { id: error.id, reason: error.reason },
              headers: new Headers({ 'Content-Type': 'application/json' }),
              status: 404,
            }
          : undefined,
    },
    handler: 'getItem',
    method: 'GET',
    path: '/items/{id}',
    request: requestSpec,
    response: responseSpec,
    security: [],
  }),
} as const;

test('createVersionedAdapterRouter connects OperationTables to adapter dispatchers and typed errors', async () => {
  type VersionId = 'v1' | 'v2';

  const dispatchGetItem = async (
    version: VersionId,
    raw: Record<string, unknown>,
  ) => {
    const id = String(raw.id ?? '');
    if (id === 'bad') {
      return {
        dispatched: false as const,
        error: { kind: 'Validation', reason: 'bad id' },
      };
    }
    if (id === 'unprojectable') {
      return {
        dispatched: true as const,
        error: { direction: 'downcast', reason: 'cannot downcast' },
        projected: false as const,
      };
    }
    if (id === 'missing') {
      return {
        dispatched: true as const,
        projected: true as const,
        value: {
          error:
            version === 'v1'
              ? new V1NotFound({ id })
              : new V2NotFound({ id, reason: 'gone' }),
          ok: false as const,
        },
      };
    }
    if (id === 'boom') {
      return {
        dispatched: true as const,
        projected: true as const,
        value: {
          error: {
            error: new XidlServerError(503, 'upstream down'),
            kind: 'Framework',
          },
          ok: false as const,
        },
      };
    }
    return {
      dispatched: true as const,
      projected: true as const,
      value: {
        ok: true as const,
        value:
          version === 'v1'
            ? { id, label: `label-${id}` }
            : { id, title: `title-${id}` },
      },
    };
  };

  const router = createVersionedAdapterRouter<VersionId>({
    bindings: {
      getItem: { dispatch: dispatchGetItem },
    },
    parameter: { in: 'header', name: 'X-Version' },
    versions: [
      {
        id: 'v1',
        operations: v1Operations,
        status: 'maintained',
        value: '2026-01-01',
      },
      {
        id: 'v2',
        operations: v2Operations,
        status: 'latest',
        value: '2026-06-01',
      },
    ],
  });

  const res1 = await router(
    new Request('http://localhost/items/7', {
      headers: { 'X-Version': '2026-01-01' },
    }),
  );
  assert.equal(res1.status, 200);
  assert.deepEqual(await res1.json(), { id: '7', label: 'label-7' });

  const res2 = await router(
    new Request('http://localhost/items/7', {
      headers: { 'X-Version': '2026-06-01' },
    }),
  );
  assert.equal(res2.status, 200);
  assert.deepEqual(await res2.json(), { id: '7', title: 'title-7' });

  const err1 = await router(
    new Request('http://localhost/items/missing', {
      headers: { 'X-Version': '2026-01-01' },
    }),
  );
  assert.equal(err1.status, 404);
  assert.deepEqual(await err1.json(), { id: 'missing' });

  const err2 = await router(
    new Request('http://localhost/items/missing', {
      headers: { 'X-Version': '2026-06-01' },
    }),
  );
  assert.equal(err2.status, 404);
  assert.deepEqual(await err2.json(), { id: 'missing', reason: 'gone' });

  assert.equal(
    (
      await router(
        new Request('http://localhost/items/bad', {
          headers: { 'X-Version': '2026-01-01' },
        }),
      )
    ).status,
    400,
  );
  assert.equal(
    (
      await router(
        new Request('http://localhost/items/unprojectable', {
          headers: { 'X-Version': '2026-01-01' },
        }),
      )
    ).status,
    422,
  );
  assert.equal(
    (
      await router(
        new Request('http://localhost/items/boom', {
          headers: { 'X-Version': '2026-06-01' },
        }),
      )
    ).status,
    503,
  );

  const single = createAdaptedRouter('v1', v1Operations, {
    getItem: { dispatch: dispatchGetItem },
  });
  const direct = await single(new Request('http://localhost/items/9'));
  assert.equal(direct.status, 200);
  assert.deepEqual(await direct.json(), { id: '9', label: 'label-9' });
});
