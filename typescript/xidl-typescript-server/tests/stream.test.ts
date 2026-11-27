import assert from 'node:assert/strict';
import test from 'node:test';
import {
  createRouter,
  defineOperation,
  StreamInterruptedError,
  XidlServerError,
} from '../src/index.ts';
import { ndjsonRequestStream, sseResponse } from '../src/stream.ts';

async function collect<T>(source: AsyncIterable<T>): Promise<T[]> {
  const values: T[] = [];
  for await (const value of source) values.push(value);
  return values;
}

test('declared stream failures use HTTP before the first item and a terminal frame after it', async () => {
  const conflict = new XidlServerError(409, 'locked');
  const match = (error: unknown) =>
    error === conflict
      ? {
          body: { msg: 'locked' },
          headers: new Headers({ ETag: 'v2' }),
          status: 409,
        }
      : undefined;
  const beforeData = async function* () {
    yield await Promise.reject<number>(conflict);
  };
  await assert.rejects(sseResponse(beforeData()), error => error === conflict);
  const rejected = await sseResponse(beforeData(), undefined, match);
  assert.equal(rejected.status, 409);
  assert.equal(rejected.headers.get('ETag'), 'v2');
  assert.deepEqual(await rejected.json(), { msg: 'locked' });
  let closed = false;
  const response = await sseResponse(
    (async function* () {
      try {
        yield 0;
        throw conflict;
      } finally {
        closed = true;
      }
    })(),
    undefined,
    match,
  );
  const text = await response.text();
  assert.equal((text.match(/event: error/g) ?? []).length, 1);
  assert.match(text, /"status":409/);
  assert.match(text, /data: 0/);
  assert.doesNotMatch(text, /event: complete/);
  assert.equal(closed, true);
});

test('serialization failure never becomes null; cancellation closes the producer', async () => {
  const failure = new Error('cannot encode item');
  const broken = {
    toJSON() {
      throw failure;
    },
  };
  await assert.rejects(
    sseResponse(
      (async function* () {
        yield broken;
      })(),
    ),
    error => error === failure,
  );
  let failureClosed = false;
  const response = await sseResponse(
    (async function* () {
      try {
        yield 1;
        yield broken;
      } finally {
        failureClosed = true;
      }
    })(),
  );
  assert(response.body);
  const failed = response.body.getReader();
  assert.equal(
    new TextDecoder().decode((await failed.read()).value),
    'event: next\ndata: 1\n\n',
  );
  await assert.rejects(failed.read(), error => error === failure);
  assert.equal(failureClosed, true);
  let closed = false;
  const cancellable = await sseResponse(
    (async function* () {
      try {
        while (true) yield 1;
      } finally {
        closed = true;
      }
    })(),
  );
  assert(cancellable.body);
  const reader = cancellable.body.getReader();
  await reader.read();
  await reader.cancel();
  assert.equal(closed, true);
});

test('an undeclared producer failure closes the reader with the same error', async () => {
  const failure = new XidlServerError(503, 'producer failure');
  let closed = false;
  const response = await sseResponse(
    (async function* () {
      try {
        yield 0;
        throw failure;
      } finally {
        closed = true;
      }
    })(),
  );
  assert(response.body);
  const reader = response.body.getReader();
  assert.equal(
    new TextDecoder().decode((await reader.read()).value),
    'event: next\ndata: 0\n\n',
  );
  await assert.rejects(reader.read(), error => error === failure);
  assert.equal(closed, true);
});

test('NDJSON requires terminal completion and does not invent a missing data value', async () => {
  function request(text: string): Request {
    return new Request('http://x/import', { body: text, method: 'POST' });
  }
  const values = await collect(
    ndjsonRequestStream(
      request(
        '{"t":"heartbeat"}\n{"t":"next","data":0}\n{"t":"next","data":false}\n{"t":"next","data":null}\n{"t":"complete"}\n{"t":"next","data":99}\n',
      ),
    ),
  );
  assert.deepEqual(values, [0, false, null]);
  for (const payload of ['', '{"t":"next"}\n', '{"t":"next","data":0}\n']) {
    await assert.rejects(collect(ndjsonRequestStream(request(payload))), {
      message: /terminal|invalid/,
      name: 'StreamInterruptedError',
    });
  }
});

test('NDJSON producer failures retain their stream type and use legal HTTP error statuses', async () => {
  type Service = { import(items: AsyncIterable<number>): Promise<void> };
  const operation = defineOperation<Service, 'import'>({
    handler: 'import',
    method: 'POST',
    path: '/import',
    request: {
      args: [],
      body: { contentType: 'application/x-ndjson', fields: [], kind: 'stream' },
      cookies: [],
      headers: [],
      kind: 'stream',
      path: [],
      query: [],
    },
    response: {
      bodyFields: [],
      bodyMode: 'none',
      contentType: 'application/json',
      cookies: [],
      headers: [],
      status: 204,
      stream: false,
    },
    security: [],
  });
  let typedFailures = 0;
  const serve = createRouter([operation], {
    async import(items) {
      try {
        await collect(items);
      } catch (error) {
        if (error instanceof StreamInterruptedError) typedFailures++;
        throw error;
      }
    },
  });
  for (const [status, expected] of [
    [0, 400],
    [200, 400],
    [304, 400],
    [409, 409],
    [999, 400],
  ]) {
    const frame = JSON.stringify({ error: { body: null, status }, t: 'error' });
    const response = await serve(
      new Request('http://test/import', {
        body: `${frame}\n`,
        headers: { 'Content-Type': 'application/x-ndjson' },
        method: 'POST',
      }),
    );
    assert.equal(response.status, expected, `producer status ${status}`);
    assert.equal((await response.json()).code, expected);
  }
  assert.equal(typedFailures, 5);
});
