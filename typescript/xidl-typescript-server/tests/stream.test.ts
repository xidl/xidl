import assert from 'node:assert/strict';
import test from 'node:test';
import { XidlServerError } from '../src/error.ts';
import { ndjsonRequestStream, sseResponse } from '../src/stream.ts';

async function collect<T>(source: AsyncIterable<T>): Promise<T[]> {
  const values: T[] = [];
  for await (const value of source) values.push(value);
  return values;
}

test('the first output determines HTTP failure or a single terminal event', async () => {
  const conflict = new XidlServerError(409, 'locked');
  await assert.rejects(
    sseResponse(
      (async function* () {
        yield await Promise.reject<number>(conflict);
      })(),
    ),
    error => error === conflict,
  );
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
  );
  const text = await response.text();
  assert.equal((text.match(/event: error/g) ?? []).length, 1);
  assert.match(text, /"status":409/);
  assert.match(text, /data: 0/);
  assert.doesNotMatch(text, /event: complete/);
  assert.equal(closed, true);
});

test('serialization failure never becomes null; cancellation closes the producer', async () => {
  const broken = {
    toJSON() {
      throw new Error('cannot encode item');
    },
  };
  await assert.rejects(
    sseResponse(
      (async function* () {
        yield broken;
      })(),
    ),
    /cannot encode/,
  );
  const response = await sseResponse(
    (async function* () {
      yield 1;
      yield broken;
    })(),
  );
  const text = await response.text();
  assert.match(text, /event: error/);
  assert.doesNotMatch(text, /data: null|event: complete/);
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
    await assert.rejects(
      collect(ndjsonRequestStream(request(payload))),
      /terminal|requires|invalid/,
    );
  }
});
