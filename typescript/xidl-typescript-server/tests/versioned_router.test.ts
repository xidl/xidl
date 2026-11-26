import assert from 'node:assert/strict';
import test from 'node:test';
import { z } from 'zod';

import {
  createRouter,
  createVersionedRouter,
  defineOperation,
} from '../src/index.js';

interface V1Service {
  getItem(id: string): { id: string; label: string };
}

interface V2Service {
  getItem(id: string): { id: string; title: string };
}

const sharedRequest = {
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

const sharedResponse = {
  bodyFields: [{ key: 'return', wireName: 'return' }],
  bodyMode: 'return' as const,
  contentType: 'application/json',
  cookies: [],
  headers: [],
  status: 200,
  stream: false,
};

const v1Op = defineOperation<V1Service, 'getItem'>({
  handler: 'getItem',
  method: 'GET',
  path: '/items/{id}',
  request: sharedRequest,
  response: sharedResponse,
  security: [],
});

const v2Op = defineOperation<V2Service, 'getItem'>({
  handler: 'getItem',
  method: 'GET',
  path: '/items/{id}',
  request: sharedRequest,
  response: sharedResponse,
  security: [],
});

test('createVersionedRouter routes by header, enforces lifecycles, echoes version and sets Vary', async () => {
  const v1Handler = createRouter([v1Op], {
    getItem: id => ({ id, label: `v1:${id}` }),
  });
  const v2Handler = createRouter([v2Op], {
    getItem: id => ({ id, title: `v2:${id}` }),
  });

  const router = createVersionedRouter({
    parameter: { in: 'header', name: 'X-Version' },
    versions: [
      { id: 'v0', status: 'retired', value: '2025-01-01' },
      {
        handler: v1Handler,
        id: 'v1',
        status: 'maintained',
        value: '2026-01-01',
      },
      { handler: v2Handler, id: 'v2', status: 'latest', value: '2026-06-01' },
    ],
  });

  const okV1 = await router(
    new Request('http://localhost/items/42', {
      headers: { 'X-Version': '2026-01-01' },
    }),
  );
  assert.equal(okV1.status, 200);
  assert.equal(okV1.headers.get('X-Version'), '2026-01-01');
  assert.equal(okV1.headers.get('Vary'), 'X-Version');
  assert.deepEqual(await okV1.json(), { id: '42', label: 'v1:42' });

  const okV2 = await router(
    new Request('http://localhost/items/42', {
      headers: { 'X-Version': '2026-06-01' },
    }),
  );
  assert.equal(okV2.status, 200);
  assert.equal(okV2.headers.get('X-Version'), '2026-06-01');
  assert.deepEqual(await okV2.json(), { id: '42', title: 'v2:42' });

  const missing = await router(new Request('http://localhost/items/42'));
  assert.equal(missing.status, 400);
  assert.equal(missing.headers.get('Vary'), 'X-Version');
  assert.deepEqual(await missing.json(), {
    code: 400,
    kind: 'missing',
    msg: "missing required version header 'X-Version'",
    supportedVersions: ['2026-01-01', '2026-06-01'],
  });

  const repeated = await router(
    new Request('http://localhost/items/42', {
      headers: { 'X-Version': '2026-01-01, 2026-06-01' },
    }),
  );
  assert.equal(repeated.status, 400);
  assert.equal((await repeated.json()).kind, 'repeated');

  const blank = await router(
    new Request('http://localhost/items/42', {
      headers: { 'X-Version': '   ' },
    }),
  );
  assert.equal(blank.status, 400);
  assert.equal((await blank.json()).kind, 'malformed');

  const unknown = await router(
    new Request('http://localhost/items/42', {
      headers: { 'X-Version': '2099-01-01' },
    }),
  );
  assert.equal(unknown.status, 400);
  assert.equal((await unknown.json()).kind, 'unknown');

  const retired = await router(
    new Request('http://localhost/items/42', {
      headers: { 'X-Version': '2025-01-01' },
    }),
  );
  assert.equal(retired.status, 410);
  assert.deepEqual(await retired.json(), {
    code: 410,
    kind: 'retired',
    msg: "version '2025-01-01' is retired",
    retiredVersion: '2025-01-01',
    supportedVersions: ['2026-01-01', '2026-06-01'],
  });
});

test('createVersionedRouter supports query, cookie, defaultVersion, Vary semantics, and custom resolver', async () => {
  const taggedHandler = (version: string) => async () =>
    new Response(JSON.stringify({ version }), {
      headers: { 'Content-Type': 'application/json', Vary: 'Accept' },
    });

  const queryRouter = createVersionedRouter({
    defaultVersion: 'v2',
    parameter: { in: 'query', name: 'api_version' },
    versions: [
      { handler: taggedHandler('v1'), id: 'v1', value: '1' },
      { handler: taggedHandler('v2'), id: 'v2', value: '2' },
    ],
  });
  const fallbackRes = await queryRouter(new Request('http://localhost/items'));
  assert.equal(fallbackRes.status, 200);
  assert.deepEqual(await fallbackRes.json(), { version: 'v2' });
  assert.equal(fallbackRes.headers.get('api_version'), null);
  assert.equal(fallbackRes.headers.get('Vary'), 'Accept');

  const explicitQuery = await queryRouter(
    new Request('http://localhost/items?api_version=1'),
  );
  assert.equal(explicitQuery.status, 200);
  assert.deepEqual(await explicitQuery.json(), { version: 'v1' });

  const repeatedQuery = await queryRouter(
    new Request('http://localhost/items?api_version=1&api_version=2'),
  );
  assert.equal(repeatedQuery.status, 400);
  assert.equal((await repeatedQuery.json()).kind, 'repeated');

  const queryEchoRouter = createVersionedRouter({
    echoHeader: true,
    parameter: { in: 'query', name: 'api_version' },
    versions: [{ handler: taggedHandler('v1'), id: 'v1', value: '1' }],
  });
  const queryEchoRes = await queryEchoRouter(
    new Request('http://localhost/items?api_version=1'),
  );
  assert.equal(queryEchoRes.status, 200);
  assert.equal(queryEchoRes.headers.get('api_version'), '1');
  assert.equal(queryEchoRes.headers.get('Vary'), 'Accept');

  const cookieRouter = createVersionedRouter({
    echoHeader: false,
    parameter: { in: 'cookie', name: 'api_version' },
    versions: [{ handler: taggedHandler('v1'), id: 'v1', value: '2026-01-01' }],
  });
  const cookieOk = await cookieRouter(
    new Request('http://localhost/items', {
      headers: { Cookie: 'other=1; api_version=2026-01-01' },
    }),
  );
  assert.equal(cookieOk.status, 200);
  assert.deepEqual(await cookieOk.json(), { version: 'v1' });
  assert.equal(cookieOk.headers.get('api_version'), null);
  assert.equal(cookieOk.headers.get('Vary'), 'Accept');

  const cookieRepeated = await cookieRouter(
    new Request('http://localhost/items', {
      headers: {
        Cookie: 'api_version=2026-01-01; api_version=2026-06-01',
      },
    }),
  );
  assert.equal(cookieRepeated.status, 400);
  assert.equal((await cookieRepeated.json()).kind, 'repeated');

  const cookieMalformed = await cookieRouter(
    new Request('http://localhost/items', {
      headers: { Cookie: 'api_version=%E0%A4%A' },
    }),
  );
  assert.equal(cookieMalformed.status, 400);
  assert.equal((await cookieMalformed.json()).kind, 'malformed');

  const cookieEchoRouter = createVersionedRouter({
    echoHeader: true,
    parameter: { in: 'cookie', name: 'api_version' },
    versions: [{ handler: taggedHandler('v1'), id: 'v1', value: '2026-01-01' }],
  });
  const cookieEchoRes = await cookieEchoRouter(
    new Request('http://localhost/items', {
      headers: { Cookie: 'api_version=2026-01-01' },
    }),
  );
  assert.equal(cookieEchoRes.status, 200);
  assert.equal(cookieEchoRes.headers.get('api_version'), '2026-01-01');
  assert.equal(cookieEchoRes.headers.get('Vary'), 'Accept, Cookie');

  const headerRouter = createVersionedRouter({
    parameter: { in: 'header', name: 'X-Version' },
    resolveVersion: raw => (raw === 'latest' ? 'v2' : undefined),
    versions: [{ handler: taggedHandler('v2'), id: 'v2', value: '2026-06-01' }],
  });
  const res = await headerRouter(
    new Request('http://localhost/items', {
      headers: { 'X-Version': 'latest' },
    }),
  );
  assert.equal(res.status, 200);
  assert.deepEqual(await res.json(), { version: 'v2' });
  assert.equal(res.headers.get('X-Version'), '2026-06-01');
  assert.equal(res.headers.get('Vary'), 'Accept, X-Version');
});

test('createVersionedRouter validates duplicate versions and invalid defaultVersion eagerly', () => {
  const noop = async () => new Response('ok');
  assert.throws(
    () =>
      createVersionedRouter({
        parameter: { in: 'header', name: 'X-Version' },
        versions: [
          { handler: noop, id: 'v1', value: '1' },
          { handler: noop, id: 'v1', value: '2' },
        ],
      }),
    /duplicate version id/,
  );
  assert.throws(
    () =>
      createVersionedRouter({
        parameter: { in: 'header', name: 'X-Version' },
        versions: [
          { handler: noop, id: 'v1', value: '1' },
          { handler: noop, id: 'v2', value: '1' },
        ],
      }),
    /duplicate version value/,
  );
  assert.throws(
    () =>
      createVersionedRouter({
        defaultVersion: 'v9',
        parameter: { in: 'header', name: 'X-Version' },
        versions: [{ handler: noop, id: 'v1', value: '1' }],
      }),
    /defaultVersion/,
  );
});
