import assert from 'node:assert/strict';
import test from 'node:test';
import { xjson } from 'xidl-typescript-codec';
import { z } from 'zod';

import {
  byteResponseStream,
  byteStreamBody,
  ndjsonBody,
  ndjsonBodyLegacy,
  sseJsonStream,
  sseJsonStreamLegacy,
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
    sseResponse('event: next\ndata: {"id":"7","display_name":"seven"}\n\n');
  const values = await collect(
    sseJsonStream(fetchImpl, 'http://x/sse', {}, schema),
  );
  assert.deepEqual(values, [{ id: '7', label: 'seven' }]);
});

test('sseJsonStream throws on error event', async () => {
  const fetchImpl = async () => sseResponse('event: error\ndata: boom\n\n');
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

test('sseJsonStreamLegacy parses error event json', async () => {
  const fetchImpl = async () =>
    sseResponse('event: error\ndata: {"msg":"oops","code":42}\n\n');
  const err = await collect(
    sseJsonStreamLegacy(fetchImpl, 'http://x/sse', {}),
  ).catch(error => error);
  assert.equal(err.message, 'oops');
  assert.equal(err.code, 42);
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
  assert.equal(text, '{"id":1}\n{"id":2}\n');
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

test('ndjsonBodyLegacy invokes cancel on the source', async () => {
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
  const stream = ndjsonBodyLegacy(source);
  const reader = stream.getReader();
  await reader.read();
  await reader.cancel();
  assert.equal(cancelled, true);
});
