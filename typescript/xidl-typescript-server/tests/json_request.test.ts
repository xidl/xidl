import assert from 'node:assert/strict';
import test from 'node:test';
import { z } from 'zod';

import { createRouter, defineOperation } from '../src/index.js';

const createItem = defineOperation<
  { createItem(name: string): { name: string } },
  'createItem'
>({
  handler: 'createItem',
  method: 'POST',
  path: '/items',
  request: {
    args: ['name'],
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
    status: 200,
    stream: false,
  },
  security: [],
});

test('malformed JSON is rejected before invoking the handler', async () => {
  let calls = 0;
  const serve = createRouter([createItem], {
    createItem(name) {
      calls++;
      return { name };
    },
  });
  for (const body of ['', '{', '{"name":']) {
    const response = await serve(
      new Request('http://test/items', {
        body,
        headers: { 'Content-Type': 'application/json' },
        method: 'POST',
      }),
    );
    assert.equal(response.status, 400);
    assert.equal((await response.json()).msg, 'invalid JSON request body');
  }
  assert.equal(calls, 0);
  const valid = await serve(
    new Request('http://test/items', {
      body: '{"name":"accepted"}',
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
    }),
  );
  assert.equal(valid.status, 200);
  assert.equal(calls, 1);
});

test('a handler SyntaxError remains a server failure', async () => {
  const serve = createRouter([createItem], {
    createItem() {
      throw new SyntaxError('handler failure');
    },
  });
  const response = await serve(
    new Request('http://test/items', {
      body: '{"name":"accepted"}',
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
    }),
  );
  assert.equal(response.status, 500);
});
