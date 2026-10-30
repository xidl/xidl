import assert from 'node:assert/strict';
import test from 'node:test';
import {
  StreamInterruptedError,
  StreamLifecycle,
  xjson,
} from 'xidl-typescript-codec';
import { z } from 'zod';

import {
  byteResponseStream,
  byteStreamBody,
  ndjsonBody,
  sseJsonStream,
} from '../src/index.js';

function sseResponse(payload: string): Response {
  const encoder = new TextEncoder();
  return new Response(
    new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(encoder.encode(payload));
        controller.close();
      },
    }),
    { status: 200 },
  );
}

async function collect<T>(source: AsyncIterable<T>): Promise<T[]> {
  const out: T[] = [];
  for await (const item of source) {
    out.push(item);
  }
  return out;
}

test('sseJsonStream yields next events and completes', async () => {
  const fetchImpl = async () =>
    sseResponse(
      'event: next\ndata: {"id":1}\n\n' +
        'event: next\ndata: {"id":2}\n\n' +
        'event: complete\ndata: done\n\n',
    );
  const values = await collect(
    sseJsonStream<{ id: number }>(fetchImpl, 'http://x/sse', {}),
  );
  assert.deepEqual(values, [{ id: 1 }, { id: 2 }]);
});

test('sseJsonStream deserializes with schema', async () => {
  const schema = z.object({
    id: z.coerce.number(),
    label: xjson(z.string(), { name: 'display_name' }),
  });
  const fetchImpl = async () =>
    sseResponse(
      'event: next\ndata: {"id":"7","display_name":"seven"}\n\nevent: complete\ndata: {}\n\n',
    );
  const values = await collect(
    sseJsonStream(fetchImpl, 'http://x/sse', {}, { schema }),
  );
  assert.deepEqual(values, [{ id: '7', label: 'seven' }]);
});

test('sseJsonStream throws on error event', async () => {
  const fetchImpl = async () =>
    sseResponse(
      'event: error\ndata: {"status":409,"body":{"code":409,"msg":"boom"}}\n\n',
    );
  await assert.rejects(
    collect(sseJsonStream(fetchImpl, 'http://x/sse', {})),
    /boom/,
  );
});

test('sseJsonStream throws on non-ok response', async () => {
  const fetchImpl = async () => new Response('nope', { status: 500 });
  await assert.rejects(
    collect(sseJsonStream(fetchImpl, 'http://x/sse', {})),
    /http error: 500/,
  );
});

test('byteResponseStream yields byte chunks', async () => {
  const encoder = new TextEncoder();
  const fetchImpl = async () =>
    new Response(
      new ReadableStream<Uint8Array>({
        start(controller) {
          controller.enqueue(encoder.encode('ab'));
          controller.enqueue(encoder.encode('c'));
          controller.close();
        },
      }),
      { status: 200 },
    );
  const chunks = await collect(
    byteResponseStream(fetchImpl, 'http://x/raw', {}),
  );
  assert.deepEqual(chunks, [[97, 98], [99]]);
});

test('ndjsonBody serializes items as ndjson', async () => {
  const source = (async function* () {
    yield { id: 1 };
    yield { id: 2 };
  })();
  const stream = ndjsonBody(source);
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let text = '';
  while (true) {
    const { value, done } = await reader.read();
    if (done) {
      break;
    }
    text += decoder.decode(value);
  }
  assert.deepEqual(
    text
      .trim()
      .split('\n')
      .map(line => JSON.parse(line)),
    [
      { data: { id: 1 }, seq: 1, t: 'next' },
      { data: { id: 2 }, seq: 2, t: 'next' },
      { t: 'complete' },
    ],
  );
});

test('byteStreamBody round-trips byte chunks', async () => {
  const source = (async function* () {
    yield [1, 2];
    yield [3];
  })();
  const stream = byteStreamBody(source);
  const reader = stream.getReader();
  const chunks: number[][] = [];
  while (true) {
    const { value, done } = await reader.read();
    if (done) {
      break;
    }
    chunks.push(Array.from(value));
  }
  assert.deepEqual(chunks, [[1, 2], [3]]);
});

test('ndjsonBody invokes cancel on the source', async () => {
  let cancelled = false;
  const source = {
    [Symbol.asyncIterator]() {
      return {
        async next() {
          return { done: false, value: { id: 1 } };
        },
        async return() {
          cancelled = true;
          return { done: true, value: undefined };
        },
      };
    },
  };
  const stream = ndjsonBody(source);
  const reader = stream.getReader();
  await reader.read();
  await reader.cancel();
  assert.equal(cancelled, true);
});

test('heartbeats and split UTF-8 preserve data; truncation never completes', async () => {
  const encoder = new TextEncoder();
  const bytes = encoder.encode(
    ': heartbeat\r\n\r\nevent:\r\ndata: "雪"\r\n\r\nevent: complete\r\ndata: {}\r\n\r\n',
  );
  const response = () =>
    new Response(
      new ReadableStream({
        start(controller) {
          for (const byte of bytes) controller.enqueue(Uint8Array.of(byte));
          controller.close();
        },
      }),
    );
  const lifecycle = new StreamLifecycle();
  assert.deepEqual(
    await collect(
      sseJsonStream(async () => response(), 'http://x', {}, { lifecycle }),
    ),
    ['雪'],
  );
  assert.equal(lifecycle.outcome, 'Completed');
  lifecycle.finish('Cancelled');
  assert.equal(lifecycle.outcome, 'Completed');
  for (const payload of [
    '',
    ': heartbeat\n\n',
    'event: next\ndata: 0\n\n',
    'event: complete\ndata: {}\n',
  ]) {
    const state = new StreamLifecycle();
    await assert.rejects(
      collect(
        sseJsonStream(
          async () => sseResponse(payload),
          'http://x',
          {},
          { lifecycle: state },
        ),
      ),
      StreamInterruptedError,
    );
    assert.equal(state.outcome, 'Interrupted');
  }
});

test('terminal failure stops before appended data and cleanup cannot replace it', async () => {
  const lifecycle = new StreamLifecycle();
  let cancelled = 0;
  const failure = new Error('declared Conflict');
  const response = new Response(
    new ReadableStream({
      cancel() {
        cancelled++;
        throw new Error('cleanup');
      },
      start(controller) {
        controller.enqueue(
          new TextEncoder().encode(
            'event: error\ndata: {"status":409,"body":{"reason":"locked"}}\n\ndata: 99\n\n',
          ),
        );
      },
    }),
  );
  await assert.rejects(
    collect(
      sseJsonStream(
        async () => response,
        'http://x',
        {},
        {
          lifecycle,
          raise: value => {
            assert.equal(value.status, 409);
            throw failure;
          },
        },
      ),
    ),
    error => error === failure,
  );
  assert.equal(lifecycle.outcome, 'Failed');
  assert.equal(cancelled, 1);
});

test('breaking consumption cancels once and a producer error emits error without complete', async () => {
  let cancelled = 0;
  const lifecycle = new StreamLifecycle();
  const response = new Response(
    new ReadableStream({
      cancel() {
        cancelled++;
      },
      start(controller) {
        controller.enqueue(new TextEncoder().encode('data: 0\n\n'));
      },
    }),
  );
  for await (const value of sseJsonStream(
    async () => response,
    'http://x',
    {},
    { lifecycle },
  )) {
    assert.equal(value, 0);
    break;
  }
  assert.equal(cancelled, 1);
  assert.equal(lifecycle.outcome, 'Cancelled');
  const source = async function* () {
    yield 0;
    throw new Error('export failed');
  };
  const frames = (await new Response(ndjsonBody(source())).text())
    .trim()
    .split('\n')
    .map(line => JSON.parse(line));
  assert.deepEqual(
    frames.map(frame => frame.t),
    ['next', 'error'],
  );
  assert.equal(frames[0].data, 0);
  assert.equal(frames[1].error.status, 500);
});
