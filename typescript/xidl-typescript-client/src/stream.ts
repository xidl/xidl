import {
  deserialize,
  SseDecoder,
  type StreamFailure,
  StreamLifecycle,
  serialize,
  type XidlSchema,
} from 'xidl-typescript-codec';

import { XidlClientError } from './error.ts';
import { decodeXidlError, parseXidlError } from './response.ts';
import type { FetchLike } from './types.ts';

export interface StreamReadOptions {
  schema?: XidlSchema;
  raise?: (failure: StreamFailure) => never | Promise<never>;
  lifecycle?: StreamLifecycle;
}

export async function* sseJsonStream<T>(
  fetchImpl: FetchLike,
  url: string,
  options: RequestInit,
  streamOptions: StreamReadOptions = {},
): AsyncIterable<T> {
  const lifecycle = streamOptions.lifecycle ?? new StreamLifecycle();
  const headers = new Headers(options.headers);
  if (!headers.has('Accept')) headers.set('Accept', 'text/event-stream');
  let resp: Response;
  try {
    resp = await fetchImpl(url, { ...options, headers });
  } catch (error) {
    lifecycle.finish(options.signal?.aborted ? 'Cancelled' : 'Interrupted');
    throw error;
  }
  if (!resp.ok) {
    lifecycle.finish('Failed');
    if (streamOptions.raise) {
      await streamOptions.raise({
        body: await resp.json().catch(() => undefined),
        status: resp.status,
      });
    }
    throw await parseXidlError(resp);
  }
  if (!resp.body) return lifecycle.interrupted('SSE response has no body');
  const reader = resp.body.getReader();
  const decoder = new TextDecoder();
  const events = new SseDecoder();
  let buffer = '';
  try {
    while (true) {
      const next = await reader.read();
      if (next.done) lifecycle.interrupted();
      buffer += decoder.decode(next.value, { stream: true });
      let end = buffer.indexOf('\n');
      while (end >= 0) {
        const line = buffer.slice(0, end).replace(/\r$/, '');
        buffer = buffer.slice(end + 1);
        const frame = events.push(line);
        if (frame?.t === 'complete') {
          lifecycle.finish('Completed');
          return;
        }
        if (frame?.t === 'error') {
          lifecycle.finish('Failed');
          if (streamOptions.raise) await streamOptions.raise(frame.error);
          throw decodeXidlError(frame.error.status, frame.error.body);
        }
        if (frame?.t === 'next') {
          yield (
            streamOptions.schema
              ? deserialize(frame.data, streamOptions.schema)
              : frame.data
          ) as T;
        }
        end = buffer.indexOf('\n');
      }
    }
  } catch (error) {
    if (!lifecycle.outcome) {
      if (options.signal?.aborted) lifecycle.finish('Cancelled');
      else lifecycle.finish('Interrupted');
    }
    throw error;
  } finally {
    lifecycle.finish('Cancelled');
    // Cleanup must not replace a received terminal error or cancellation.
    try {
      await reader.cancel().catch(() => undefined);
    } finally {
      reader.releaseLock();
    }
  }
}

export async function* byteResponseStream(
  fetchImpl: FetchLike,
  url: string,
  options: RequestInit,
): AsyncIterable<number[]> {
  const resp = await fetchImpl(url, options);
  if (!resp.ok) {
    throw await parseXidlError(resp);
  }
  if (!resp.body) {
    throw new XidlClientError('byte response has no body', 500, resp.status);
  }
  const reader = resp.body.getReader();
  while (true) {
    const next = await reader.read();
    if (next.done) {
      return;
    }
    yield Array.from(next.value);
  }
}

export function ndjsonBody<T>(
  source: AsyncIterable<T>,
  schema?: XidlSchema,
): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  const iterator = source[Symbol.asyncIterator]();
  let seq = 0;
  return new ReadableStream<Uint8Array>({
    async cancel() {
      await iterator.return?.();
    },
    async pull(controller) {
      try {
        const next = await iterator.next();
        if (next.done) {
          controller.enqueue(encoder.encode('{"t":"complete"}\n'));
          controller.close();
          return;
        }
        const data = schema ? serialize(next.value, schema) : next.value;
        if (data === undefined)
          throw new Error('NDJSON item has no JSON representation');
        controller.enqueue(
          encoder.encode(
            `${JSON.stringify({ data, seq: ++seq, t: 'next' })}\n`,
          ),
        );
      } catch (error) {
        const failure = {
          body: { code: 500, msg: String(error) },
          status: 500,
        };
        controller.enqueue(
          encoder.encode(`${JSON.stringify({ error: failure, t: 'error' })}\n`),
        );
        controller.close();
        await iterator.return?.().catch(() => undefined);
      }
    },
  });
}

export function byteStreamBody(
  source: AsyncIterable<number[]>,
): ReadableStream<Uint8Array> {
  const iterator = source[Symbol.asyncIterator]();
  return new ReadableStream<Uint8Array>({
    async pull(controller) {
      const next = await iterator.next();
      if (next.done) {
        controller.close();
        return;
      }
      controller.enqueue(Uint8Array.from(next.value));
    },
  });
}
