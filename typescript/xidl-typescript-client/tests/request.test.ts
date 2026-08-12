import assert from 'node:assert/strict';
import test from 'node:test';

import {
  appendCookie,
  appendHeader,
  appendParam,
  applyClientAuth,
  encodeRequestBody,
} from '../src/index.js';

test('appendParam appends single and multi values', () => {
  const query = new URLSearchParams();
  appendParam(query, 'id', 7, false, false);
  appendParam(query, 'tag', ['a', 'b'], false, true);
  appendParam(query, 'opt', null, true, false);
  appendParam(query, 'missing', null, false, false);
  assert.equal(query.toString(), 'id=7&tag=a&tag=b&missing=');
});

test('appendParam encodes object values as json', () => {
  const query = new URLSearchParams();
  appendParam(query, 'filter', { x: 1 }, false, false);
  assert.equal(query.get('filter'), '{"x":1}');
});

test('appendHeader sets and appends multi values', () => {
  const headers = new Headers();
  appendHeader(headers, 'X-Rate', 3, false, false);
  appendHeader(headers, 'X-Tag', ['a', 'b'], false, true);
  appendHeader(headers, 'X-Opt', undefined, true, false);
  assert.equal(headers.get('X-Rate'), '3');
  assert.deepEqual(headers.getSetCookie && headers.get('X-Tag'), 'a, b');
  assert.equal(headers.get('X-Opt'), null);
});

test('appendCookie builds cookie pairs with uri encoding', () => {
  const cookies: string[] = [];
  appendCookie(cookies, 'session', 'a b', false, false);
  appendCookie(cookies, 'tag', ['x', 'y'], false, true);
  appendCookie(cookies, 'opt', null, true, false);
  assert.deepEqual(cookies, ['session=a%20b', 'tag=x', 'tag=y']);
});

test('applyClientAuth applies basic auth', () => {
  const headers = new Headers();
  applyClientAuth(
    '',
    new URLSearchParams(),
    headers,
    { kind: 'basic', password: 'p', username: 'u' },
    [{ kind: 'basic' }],
  );
  assert.match(headers.get('Authorization') ?? '', /^Basic /);
  assert.equal(atob((headers.get('Authorization') ?? '').slice(6)), 'u:p');
});

test('applyClientAuth applies bearer auth', () => {
  const headers = new Headers();
  applyClientAuth(
    '',
    new URLSearchParams(),
    headers,
    { kind: 'bearer', token: 'tok' },
    [{ kind: 'bearer' }],
  );
  assert.equal(headers.get('Authorization'), 'Bearer tok');
});

test('applyClientAuth applies api_key in header by default', () => {
  const headers = new Headers();
  applyClientAuth(
    '',
    new URLSearchParams(),
    headers,
    { kind: 'api_key', value: 'k' },
    [{ kind: 'api_key' }],
  );
  assert.equal(headers.get('x-api-key'), 'k');
});

test('applyClientAuth applies api_key in query', () => {
  const query = new URLSearchParams();
  applyClientAuth(
    '',
    query,
    new Headers(),
    { kind: 'api_key', location: 'query', name: 'key', value: 'k' },
    [{ kind: 'api_key', location: 'query', name: 'key' }],
  );
  assert.equal(query.get('key'), 'k');
});

test('applyClientAuth applies api_key in cookie', () => {
  const headers = new Headers();
  applyClientAuth(
    '',
    new URLSearchParams(),
    headers,
    { kind: 'api_key', location: 'cookie', name: 'sid', value: 'v' },
    [{ kind: 'api_key', location: 'cookie', name: 'sid' }],
  );
  assert.equal(headers.get('Cookie'), 'sid=v');
});

test('applyClientAuth ignores auth when no requirement matches', () => {
  const headers = new Headers();
  applyClientAuth(
    '',
    new URLSearchParams(),
    headers,
    { kind: 'bearer', token: 'tok' },
    [{ kind: 'basic' }],
  );
  assert.equal(headers.get('Authorization'), null);
});

test('encodeRequestBody encodes json with schema', () => {
  const body = encodeRequestBody({ name: 'x' }, 'application/json', {});
  assert.equal(body, '{"name":"x"}');
});

test('encodeRequestBody encodes form data', () => {
  const body = encodeRequestBody(
    { a: '1', b: ['2', '3'] },
    'application/x-www-form-urlencoded',
    {},
  );
  assert.ok(body instanceof URLSearchParams);
  assert.equal((body as URLSearchParams).toString(), 'a=1&b=2&b=3');
});

test('encodeRequestBody uses custom codec', () => {
  const body = encodeRequestBody('raw', 'application/cbor', {
    'application/cbor': {
      encode: value => new TextEncoder().encode(String(value)),
    },
  });
  assert.ok(body instanceof Uint8Array);
});

test('encodeRequestBody throws for unsupported content type', () => {
  assert.throws(
    () => encodeRequestBody({}, 'application/unknown', {}),
    /unsupported request content type/,
  );
});
