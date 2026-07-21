import assert from 'node:assert/strict';
import test from 'node:test';

import { z } from 'zod';

import { createRouter, defineOperation } from '../src/index.js';

interface ItemServer {
  createItem(request: { name: string }): { name: string };
  getItem(request: { id: number }): { id: number };
}

const getItem = defineOperation<ItemServer, 'getItem'>({
  handler: 'getItem',
  method: 'GET',
  path: '/items/{id}',
  request: {
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
    stream: false,
  },
  security: [],
});

const createItem = defineOperation<ItemServer, 'createItem'>({
  handler: 'createItem',
  method: 'POST',
  path: '/items',
  request: {
    body: {
      contentType: 'application/json',
      fields: [{ key: 'name', wireName: 'name' }],
      kind: 'value',
    },
    cookies: [],
    headers: [],
    kind: 'object',
    path: [],
    query: [],
    schema: z.object({ name: z.string() }),
  },
  response: {
    bodyFields: [{ key: 'return', wireName: 'return' }],
    bodyMode: 'return',
    contentType: 'application/json',
    cookies: [],
    headers: [],
    stream: false,
  },
  security: [],
});

const router = createRouter([getItem, createItem], {
  createItem: request => request,
  getItem: request => request,
});

test('createRouter dispatches by method and path', async () => {
  const response = await router(new Request('http://localhost/items/7'));

  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { id: 7 });
});

test('createRouter dispatches operation route aliases', async () => {
  const handler = createRouter(
    [{ ...getItem, paths: ['/items/{id}', '/legacy/items/{id}'] }],
    {
      createItem: request => request,
      getItem: request => request,
    },
  );

  const response = await handler(
    new Request('http://localhost/legacy/items/7'),
  );

  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { id: 7 });
});

test('createRouter returns a typed not-found response', async () => {
  const response = await router(new Request('http://localhost/missing'));

  assert.equal(response.status, 404);
  assert.deepEqual(await response.json(), {
    code: 404,
    msg: 'route not found',
  });
});

test('createRouter returns method not allowed for a known path', async () => {
  const response = await router(
    new Request('http://localhost/items/7', { method: 'POST' }),
  );

  assert.equal(response.status, 405);
  assert.equal(response.headers.get('Allow'), 'GET');
  assert.deepEqual(await response.json(), {
    code: 405,
    msg: 'method not allowed',
  });
});

test('createRouter enforces Accept headers', async () => {
  const response = await router(
    new Request('http://localhost/items/7', {
      headers: { Accept: 'text/plain' },
    }),
  );

  assert.equal(response.status, 406);
});

test('createRouter enforces request content types', async () => {
  const response = await router(
    new Request('http://localhost/items', {
      body: 'Alice',
      headers: { 'Content-Type': 'text/plain' },
      method: 'POST',
    }),
  );

  assert.equal(response.status, 415);
});

test('createRouter falls back to primitive return values', async () => {
  interface PrimitiveServer {
    getValue(request: { id: number }): number;
  }
  const operation = defineOperation<PrimitiveServer, 'getValue'>({
    ...getItem,
    handler: 'getValue',
    response: {
      ...getItem.response,
      bodyFields: [{ key: '_return', wireName: 'return' }],
      contentType: 'text/plain',
    },
  });
  const handler = createRouter([operation], {
    getValue: request => request.id,
  });

  const response = await handler(new Request('http://localhost/items/7'));

  assert.equal(await response.text(), '7');
});

test('createRouter handles raw byte request and response streams', async () => {
  interface ByteServer {
    download(): AsyncIterable<number[]>;
    upload(stream: AsyncIterable<number[]>): Promise<string>;
  }
  const download = defineOperation<ByteServer, 'download'>({
    ...getItem,
    handler: 'download',
    path: '/download',
    request: {
      body: { contentType: '', fields: [], kind: 'none' },
      cookies: [],
      headers: [],
      kind: 'none',
      path: [],
      query: [],
    },
    response: {
      bodyFields: [{ key: 'return', wireName: 'return' }],
      bodyMode: 'return',
      contentType: 'application/octet-stream',
      cookies: [],
      headers: [],
      stream: true,
    },
  });
  const upload = defineOperation<ByteServer, 'upload'>({
    ...createItem,
    handler: 'upload',
    path: '/upload',
    request: {
      body: {
        contentType: 'application/octet-stream',
        fields: [],
        kind: 'stream',
      },
      cookies: [],
      headers: [],
      kind: 'stream',
      path: [],
      query: [],
    },
    response: {
      bodyFields: [{ key: 'return', wireName: 'return' }],
      bodyMode: 'return',
      contentType: 'text/plain',
      cookies: [],
      headers: [],
      stream: false,
    },
  });
  const handler = createRouter([download, upload], {
    async *download() {
      yield Array.from(new TextEncoder().encode('hello '));
      yield Array.from(new TextEncoder().encode('world'));
    },
    async upload(stream) {
      const chunks: number[] = [];
      for await (const chunk of stream) {
        chunks.push(...chunk);
      }
      return new TextDecoder().decode(Uint8Array.from(chunks));
    },
  });

  const downloadResponse = await handler(
    new Request('http://localhost/download'),
  );
  assert.equal(
    downloadResponse.headers.get('Content-Type'),
    'application/octet-stream',
  );
  assert.equal(await downloadResponse.text(), 'hello world');

  const uploadResponse = await handler(
    new Request('http://localhost/upload', {
      body: new TextEncoder().encode('uploaded bytes'),
      headers: { 'Content-Type': 'application/octet-stream' },
      method: 'POST',
    }),
  );
  assert.equal(await uploadResponse.text(), 'uploaded bytes');
});
