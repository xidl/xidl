import assert from 'node:assert/strict';
import test from 'node:test';
import {
  type BoundInterface,
  createAdaptedRouter,
  createVersionedAdapterRouter,
  defineOperation,
} from '../src/index.js';

type LookupService = Record<string, () => { id: number }>;

const binding = {
  dispatch: () =>
    ({
      dispatched: true,
      projected: true,
      value: { ok: true, value: { id: 7 } },
    }) as const,
};

function operation(handler: string, path: string, paths?: string[]) {
  return defineOperation<LookupService, string>({
    handler,
    method: 'GET',
    path,
    paths,
    request: {
      args: [],
      body: { contentType: '', fields: [], kind: 'none' },
      cookies: [],
      headers: [],
      kind: 'none',
      path: [],
      query: [],
    },
    response: {
      bodyFields: [],
      bodyMode: 'return',
      contentType: 'application/json',
      cookies: [],
      headers: [],
      status: 200,
      stream: false,
    },
    security: [],
  });
}

function mount(...interfaces: BoundInterface<'v1'>['operations'][]) {
  return createVersionedAdapterRouter({
    parameter: { in: 'header', name: 'X-Version' },
    versions: [
      {
        id: 'v1',
        interfaces: interfaces.map(operations => ({
          operations,
          version: 'v1',
        })),
        value: '2026-01-01',
      },
    ],
  });
}

test('versioned adapter routers name the conflicting route alias across tables', () => {
  assert.throws(
    () =>
      mount(
        [{ binding, operation: operation('lookup', '/shared/{id}') }],
        [
          {
            binding,
            operation: operation('lookup', '/secondary/{slug}', [
              '/secondary/{slug}',
              '/shared/{slug}',
            ]),
          },
        ],
      ),
    { message: 'conflicting GET route /shared/{slug} across operation tables' },
  );
});

test('versioned adapter routers reject conflicting routes inside one table', () => {
  assert.throws(
    () =>
      mount([
        { binding, operation: operation('lookupItem', '/items/{id}') },
        { binding, operation: operation('lookupSlug', '/items/{slug}') },
      ]),
    {
      message: 'conflicting GET route /items/{slug} within an operation table',
    },
  );
});

test('versioned adapter routers reject a repeated handler inside one table', () => {
  assert.throws(
    () =>
      mount([
        { binding, operation: operation('lookup', '/items') },
        { binding, operation: operation('lookup', '/accounts') },
      ]),
    { message: "duplicate adapter handler 'lookup' in an operation table" },
  );
});

test('a single adapter router accepts same-handler aliases in separate descriptors', async () => {
  const route = createAdaptedRouter(
    'v1',
    [operation('lookup', '/items'), operation('lookup', '/legacy/items')],
    { lookup: binding },
  );
  for (const path of ['/items', '/legacy/items']) {
    const response = await route(new Request(`http://localhost${path}`));
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { id: 7 });
  }
});
