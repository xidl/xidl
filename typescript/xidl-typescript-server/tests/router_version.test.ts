import assert from 'node:assert/strict';
import test from 'node:test';
import { z } from 'zod';

import {
  createRouter,
  defineOperation,
  XidlServerError,
} from '../src/index.js';

interface ItemServer {
  createItem(name: string): { name: string };
  getItem(id: number): { id: number };
}

const getItem = defineOperation<ItemServer, 'getItem'>({
  handler: 'getItem',
  method: 'GET',
  path: '/items/{id}',
  request: {
    args: ['id'],
    body: { contentType: '', fields: [], kind: 'none' },
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
    schema: z.object({ id: z.coerce.number() }),
  },
  response: {
    bodyFields: [{ key: 'return', wireName: 'return' }],
    bodyMode: 'return',
    contentType: 'application/json',
    cookies: [],
    headers: [],
    status: 200,
    stream: false,
  },
  security: [],
});

test('fixed operation headers survive success, parsing, authorization and handler errors', async () => {
  const typed = new Error('conflict');
  let calls = 0;
  const operation = defineOperation<ItemServer, 'getItem'>({
    ...getItem,
    errors: {
      match: error =>
        error === typed
          ? {
              body: { conflict: true },
              headers: new Headers({
                ETag: 'revision-2',
                'X-Version': 'spoofed',
              }),
              status: 409,
            }
          : undefined,
    },
    response: { ...getItem.response, fixedHeaders: { 'X-Version': 'v2' } },
  });
  const handler = createRouter(
    [operation],
    {
      createItem: name => ({ name }),
      getItem: id => {
        calls++;
        if (id === 2) throw typed;
        if (id === 3) throw new Error('handler failed');
        return { id };
      },
    },
    {
      authorize: request => {
        if (request.headers.has('Reject'))
          throw new XidlServerError(401, 'denied');
      },
    },
  );
  for (const [path, status, reject] of [
    ['1', 200, false],
    ['2', 409, false],
    ['3', 500, false],
    ['not-a-number', 400, false],
    ['%ZZ', 400, false],
    ['1', 401, true],
  ] as const) {
    const headers = new Headers({ 'X-Version': 'v1' });
    if (reject) headers.set('Reject', 'yes');
    const response = await handler(
      new Request(`http://localhost/items/${path}`, { headers }),
    );
    assert.equal(response.status, status, path);
    assert.equal(response.headers.get('X-Version'), 'v2', path);
    if (status === 409) {
      assert.equal(response.headers.get('ETag'), 'revision-2');
      assert.deepEqual(await response.json(), { conflict: true });
    }
  }
  assert.equal(
    calls,
    3,
    'invalid and unauthorized requests never invoke business handlers',
  );
  const custom = createRouter(
    [operation],
    {
      createItem: name => ({ name }),
      getItem: () => {
        throw new Error('offline');
      },
    },
    {
      onError: () =>
        new Response('offline', {
          headers: { 'X-Version': 'old' },
          status: 503,
        }),
    },
  );
  const response = await custom(new Request('http://localhost/items/1'));
  assert.equal(response.status, 503);
  assert.equal(response.headers.get('X-Version'), 'v2');
  assert.equal(await response.text(), 'offline');
  for (const [path, method, status] of [
    ['missing', 'GET', 404],
    ['items/1', 'POST', 405],
  ] as const) {
    const unmatched = await handler(
      new Request(`http://localhost/${path}`, { method }),
    );
    assert.equal(unmatched.status, status);
    assert.equal(
      unmatched.headers.get('X-Version'),
      null,
      'no operation selected',
    );
  }
});

test('router decodes captured paths exactly once, including catch-all segments', async () => {
  for (const catchAll of [false, true]) {
    const operation = defineOperation<{ read(id: string): string }, 'read'>({
      ...getItem,
      handler: 'read',
      path: catchAll ? '/files/{*id}' : '/files/{id}',
      request: { ...getItem.request, schema: z.object({ id: z.string() }) },
      response: { ...getItem.response, contentType: 'text/plain' },
    });
    const handler = createRouter([operation], { read: id => id });
    for (const [path, expected] of [
      ['%252F', '%2F'],
      ['%25', '%'],
      ['a%2Fb', 'a/b'],
    ]) {
      const suffix = catchAll ? '/tail%252F' : '';
      const response = await handler(
        new Request(`http://localhost/files/${path}${suffix}`),
      );
      assert.equal(response.status, 200);
      assert.equal(
        await response.text(),
        expected + (catchAll ? '/tail%2F' : ''),
      );
    }
  }
});
