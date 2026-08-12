import assert from 'node:assert/strict';
import test from 'node:test';

import { z } from 'zod';

import {
  buildResponsePayload,
  decodeOptionalResponseBody,
  decodeResponseBody,
  joinUrl,
  parseScalar,
  parseXidlError,
  readResponseHeader,
} from '../src/index.js';

test('decodeResponseBody decodes json with schema', async () => {
  const resp = new Response(JSON.stringify({ id: 3 }), {
    headers: { 'Content-Type': 'application/json' },
  });
  const value = await decodeResponseBody(
    resp,
    'application/json',
    {},
    z.object({ id: z.number() }),
  );
  assert.deepEqual(value, { id: 3 });
});

test('decodeResponseBody decodes text', async () => {
  const resp = new Response('hello', {
    headers: { 'Content-Type': 'text/plain' },
  });
  const value = await decodeResponseBody(resp, 'text/plain', {});
  assert.equal(value, 'hello');
});

test('decodeResponseBody uses custom codec', async () => {
  const resp = new Response('cbor');
  const value = await decodeResponseBody(resp, 'application/cbor', {
    'application/cbor': { decode: async r => r.text() },
  });
  assert.equal(value, 'cbor');
});

test('decodeResponseBody throws for unsupported content type', async () => {
  const resp = new Response('x');
  await assert.rejects(
    decodeResponseBody(resp, 'application/unknown', {}),
    /unsupported response content type/,
  );
});

test('decodeOptionalResponseBody returns undefined on 204', async () => {
  const resp = new Response(null, { status: 204 });
  const value = await decodeOptionalResponseBody(resp, 'application/json', {});
  assert.equal(value, undefined);
});

test('buildResponsePayload merges body, headers and cookies', () => {
  const headers = new Headers();
  headers.set('X-Total', '10');
  const resp = new Response(null, { headers });
  const payload = buildResponsePayload(
    { id: 1 },
    resp,
    'object',
    [{ isMulti: false, key: 'total', name: 'X-Total', optional: true }],
    [],
  );
  assert.deepEqual(payload, { id: 1, total: 10 });
});

test('readResponseHeader parses multi values', () => {
  const headers = new Headers();
  headers.set('X-Tag', 'a, b');
  assert.deepEqual(readResponseHeader(headers, 'X-Tag', true), ['a', 'b']);
});

test('parseScalar coerces primitives', () => {
  assert.equal(parseScalar('true'), true);
  assert.equal(parseScalar('42'), 42);
  assert.deepEqual(parseScalar('{"a":1}'), { a: 1 });
  assert.equal(parseScalar('raw'), 'raw');
});

test('parseXidlError parses structured error body', async () => {
  const resp = new Response(JSON.stringify({ code: 42, msg: 'boom' }), {
    status: 400,
  });
  const err = await parseXidlError(resp);
  assert.equal(err.code, 42);
  assert.equal(err.status, 400);
  assert.equal(err.message, 'boom');
  assert.ok(err instanceof Error);
});

test('parseXidlError falls back to http error', async () => {
  const resp = new Response('nope', { status: 500 });
  const err = await parseXidlError(resp);
  assert.equal(err.message, 'http error: 500');
  assert.equal(err.code, 500);
});

test('joinUrl joins base and path', () => {
  assert.equal(joinUrl('http://x/api', '/v1'), 'http://x/api/v1');
  assert.equal(joinUrl('http://x/api/', '/v1'), 'http://x/api/v1');
  assert.equal(joinUrl('http://x', 'v1'), 'http://x/v1');
  assert.equal(joinUrl('http://x/', 'v1'), 'http://x/v1');
});
