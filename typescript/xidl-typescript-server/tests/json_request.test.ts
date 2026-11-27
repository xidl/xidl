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

test('handler exceptions keep their identity even when they look like decoding failures', async () => {
  const validation = z.string().safeParse(0);
  assert(!validation.success);
  const failures = [
    new SyntaxError('handler failure'),
    { issues: [], name: 'UpstreamProblem' },
    validation.error,
  ];
  for (const failure of failures) {
    const serve = createRouter([createItem], {
      createItem() {
        throw failure;
      },
    });
    await assert.rejects(
      serve(
        new Request('http://test/items', {
          body: '{"name":"accepted"}',
          headers: { 'Content-Type': 'application/json' },
          method: 'POST',
        }),
      ),
      error => error === failure,
    );
  }
});

test('a union validation error without the local Zod prototype produces a bad request', async () => {
  const text = z.string();
  const validate = text.safeParse.bind(text);
  text.safeParse = value => {
    const result = validate(value);
    if (result.success) return result;
    // A different Zod copy can supply issues without sharing the error prototype.
    return {
      error: { issues: result.error.issues, name: result.error.name },
      success: false,
    } as typeof result;
  };
  const operation = {
    ...createItem,
    request: {
      ...createItem.request,
      schema: z.object({ name: z.union([z.literal('anonymous'), text]) }),
    },
  };
  let calls = 0;
  const serve = createRouter([operation], {
    createItem(name) {
      calls++;
      return { name };
    },
  });
  const response = await serve(
    new Request('http://test/items', {
      body: '{"name":42}',
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
    }),
  );
  assert.equal(response.status, 400);
  const failure = await response.json();
  assert.equal(failure.msg, 'invalid request');
  assert.deepEqual(
    failure.detail.map(({ code, expected, path }) => ({
      code,
      expected,
      path,
    })),
    [{ code: 'invalid_type', expected: 'string', path: [] }],
  );
  assert.equal(calls, 0);
});
