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

test('createVersionedRouter routes by header, reports retired wire values and sets Vary', async () => {
  const v1Handler = createRouter([v1Op], {
    getItem: id => ({ id, label: `v1:${id}` }),
  });
  const v2Handler = createRouter([v2Op], {
    getItem: id => ({ id, title: `v2:${id}` }),
  });

  const router = createVersionedRouter({
    parameter: { in: 'header', name: 'X-Version' },
    retired: ['2025-01-01'],
    versions: [
      {
        handler: v1Handler,
        id: 'v1',
        value: '2026-01-01',
      },
      { handler: v2Handler, id: 'v2', value: '2026-06-01' },
    ],
  });

  const okV1 = await router(
    new Request('http://localhost/items/42', {
      headers: { 'X-Version': '2026-01-01' },
    }),
  );
  assert.equal(okV1.status, 200);
  assert.equal(okV1.headers.get('X-Version'), null);
  assert.equal(okV1.headers.get('Vary'), 'X-Version');
  assert.deepEqual(await okV1.json(), { id: '42', label: 'v1:42' });

  const okV2 = await router(
    new Request('http://localhost/items/42', {
      headers: { 'X-Version': '2026-06-01' },
    }),
  );
  assert.equal(okV2.status, 200);
  assert.equal(okV2.headers.get('X-Version'), null);
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

  const unmounted = await router(
    new Request('http://localhost/items/42', {
      headers: { 'X-Version': '2028-06-01' },
    }),
  );
  assert.equal(unmounted.status, 400);
  assert.deepEqual(await unmounted.json(), {
    code: 400,
    kind: 'unknown',
    msg: "unsupported version '2028-06-01'",
    supportedVersions: ['2026-01-01', '2026-06-01'],
  });

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

test('createVersionedRouter supports query, cookie, defaultVersion and composed handlers', async () => {
  const taggedHandler = (version: string) => async () =>
    new Response(JSON.stringify({ version }), {
      headers: { 'Content-Type': 'application/json', Vary: 'Accept' },
    });
  const withVersion =
    (version: string, name: string, wire: string) => async () => {
      const response = await taggedHandler(version)();
      response.headers.set(name, wire);
      return response;
    };

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
  assert.equal(repeatedQuery.headers.get('Vary'), null);
  assert.equal((await repeatedQuery.json()).kind, 'repeated');

  const queryEchoRouter = createVersionedRouter({
    parameter: { in: 'query', name: 'api_version' },
    versions: [
      { handler: withVersion('v1', 'api_version', '1'), id: 'v1', value: '1' },
    ],
  });
  const queryEchoRes = await queryEchoRouter(
    new Request('http://localhost/items?api_version=1'),
  );
  assert.equal(queryEchoRes.status, 200);
  assert.equal(queryEchoRes.headers.get('api_version'), '1');
  assert.equal(queryEchoRes.headers.get('Vary'), 'Accept');

  const cookieRouter = createVersionedRouter({
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
  assert.equal(cookieOk.headers.get('Vary'), 'Accept, Cookie');

  const cookieRepeated = await cookieRouter(
    new Request('http://localhost/items', {
      headers: {
        Cookie: 'api_version=2026-01-01; api_version=2026-06-01',
      },
    }),
  );
  assert.equal(cookieRepeated.status, 400);
  assert.equal(cookieRepeated.headers.get('Vary'), 'Cookie');
  assert.equal((await cookieRepeated.json()).kind, 'repeated');

  const cookieMalformed = await cookieRouter(
    new Request('http://localhost/items', {
      headers: { Cookie: 'api_version=%E0%A4%A' },
    }),
  );
  assert.equal(cookieMalformed.status, 400);
  assert.equal(cookieMalformed.headers.get('Vary'), 'Cookie');
  assert.equal((await cookieMalformed.json()).kind, 'malformed');

  const cookieEchoRouter = createVersionedRouter({
    parameter: { in: 'cookie', name: 'api_version' },
    versions: [
      {
        handler: withVersion('v1', 'api_version', '2026-01-01'),
        id: 'v1',
        value: '2026-01-01',
      },
    ],
  });
  const cookieEchoRes = await cookieEchoRouter(
    new Request('http://localhost/items', {
      headers: { Cookie: 'api_version=2026-01-01' },
    }),
  );
  assert.equal(cookieEchoRes.status, 200);
  assert.equal(cookieEchoRes.headers.get('api_version'), '2026-01-01');
  assert.equal(cookieEchoRes.headers.get('Vary'), 'Accept, Cookie');

  const exactHeaderRouter = createVersionedRouter({
    parameter: { in: 'header', name: 'X-Version' },
    retired: ['2025-01-01'],
    versions: [
      {
        handler: withVersion('v2', 'X-Version', '2026-06-01'),
        id: 'v2',
        value: '2026-06-01',
      },
    ],
  });
  const headerRouter = (request: Request) => {
    if (request.headers.get('X-Version') !== 'latest') {
      return exactHeaderRouter(request);
    }
    const headers = new Headers(request.headers);
    headers.set('X-Version', '2026-06-01');
    return exactHeaderRouter(new Request(request, { headers }));
  };
  const res = await headerRouter(
    new Request('http://localhost/items', {
      headers: { 'X-Version': 'latest' },
    }),
  );
  assert.equal(res.status, 200);
  assert.deepEqual(await res.json(), { version: 'v2' });
  assert.equal(res.headers.get('X-Version'), '2026-06-01');
  assert.equal(res.headers.get('Vary'), 'Accept, X-Version');

  const retired = await headerRouter(
    new Request('http://localhost/items', {
      headers: { 'X-Version': '2025-01-01' },
    }),
  );
  assert.equal(retired.status, 410);
  assert.equal(retired.headers.get('Vary'), 'X-Version');
  assert.deepEqual(await retired.json(), {
    code: 410,
    kind: 'retired',
    msg: "version '2025-01-01' is retired",
    retiredVersion: '2025-01-01',
    supportedVersions: ['2026-06-01'],
  });
});

test('createVersionedRouter rejects duplicated retired wire values and mounted overlaps', () => {
  const handler = async () => new Response('ok');
  const parameter = { in: 'header', name: 'X-Version' } as const;
  assert.throws(
    () =>
      createVersionedRouter({
        parameter,
        retired: ['1'],
        versions: [{ handler, id: 'v1', value: '1' }],
      }),
    /duplicate version value '1'/,
  );
  assert.throws(
    () =>
      createVersionedRouter({
        parameter,
        retired: ['old', 'old'],
        versions: [{ handler, id: 'v1', value: '1' }],
      }),
    /duplicate version value 'old'/,
  );
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
      createVersionedRouter<string>({
        defaultVersion: 'v9',
        parameter: { in: 'header', name: 'X-Version' },
        versions: [{ handler: noop, id: 'v1', value: '1' }],
      }),
    /defaultVersion/,
  );
});
