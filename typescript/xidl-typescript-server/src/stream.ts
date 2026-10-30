import {
  decodeStreamFrame,
  deserialize,
  type StreamFailure,
  StreamInterruptedError,
  StreamLifecycle,
  serialize,
  type XidlSchema,
} from 'xidl-typescript-codec';

import { projectError, XidlServerError } from './error.ts';

export async function* ndjsonRequestStream<T>(
  request: Request,
  schema?: XidlSchema,
): AsyncIterable<T> {
  const lifecycle = new StreamLifecycle();
  if (!request.body) return lifecycle.interrupted('NDJSON request has no body');
  const reader = request.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  try {
    while (true) {
      const next = await reader.read();
      if (next.done)
        lifecycle.interrupted('NDJSON request ended without a terminal frame');
      buffer += decoder.decode(next.value, { stream: true });
      let end = buffer.indexOf('\n');
      while (end >= 0) {
        const line = buffer.slice(0, end).trim();
        buffer = buffer.slice(end + 1);
        if (line) {
          const frame = decodeStreamFrame(JSON.parse(line));
          if (frame.t === 'complete') {
            lifecycle.finish('Completed');
            return;
          }
          if (frame.t === 'error') {
            lifecycle.finish('Failed');
            throw new XidlServerError(
              frame.error.status,
              'NDJSON producer failed',
            );
          }
          if (frame.t === 'next')
            yield (
              schema
                ? schema.parse(deserialize(frame.data, schema))
                : frame.data
            ) as T;
        }
        end = buffer.indexOf('\n');
      }
    }
  } catch (error) {
    if (!lifecycle.outcome) lifecycle.finish('Interrupted');
    if (
      lifecycle.outcome === 'Interrupted' &&
      !(error instanceof StreamInterruptedError)
    ) {
      throw new StreamInterruptedError(
        `invalid NDJSON request: ${String(error)}`,
      );
    }
    throw error;
  } finally {
    lifecycle.finish('Cancelled');
    try {
      await reader.cancel().catch(() => undefined);
    } finally {
      reader.releaseLock();
    }
  }
}

export async function* byteRequestStream(
  request: Request,
): AsyncIterable<number[]> {
  if (!request.body) {
    return;
  }
  const reader = request.body.getReader();
  while (true) {
    const next = await reader.read();
    if (next.done) {
      return;
    }
    yield Array.from(next.value);
  }
}

export async function sseResponse<T>(
  source: AsyncIterable<T>,
  schema?: XidlSchema,
  mapError: (error: unknown) => StreamFailure = projectError,
): Promise<Response> {
  const iterator = source[Symbol.asyncIterator]();
  const encoder = new TextEncoder();
  const lifecycle = new StreamLifecycle();
  const encode = (value: T): Uint8Array => {
    const wire = schema ? serialize(value, schema) : value;
    const json = JSON.stringify(wire);
    if (json === undefined)
      throw new Error('SSE item has no JSON representation');
    return encoder.encode(`event: next\ndata: ${json}\n\n`);
  };
  let first: Uint8Array;
  try {
    const item = await iterator.next();
    if (item.done) {
      lifecycle.finish('Completed');
      first = encoder.encode('event: complete\ndata: {}\n\n');
    } else first = encode(item.value);
  } catch (error) {
    // Preserve the first-output exception if producer cleanup also fails.
    await iterator.return?.().catch(() => undefined);
    throw error;
  }
  let pending: Uint8Array | undefined = first;
  return new Response(
    new ReadableStream<Uint8Array>({
      async cancel() {
        lifecycle.finish('Cancelled');
        await iterator.return?.();
      },
      async pull(controller) {
        if (pending) {
          controller.enqueue(pending);
          pending = undefined;
          if (lifecycle.outcome) controller.close();
          return;
        }
        try {
          const item = await iterator.next();
          if (item.done) {
            lifecycle.finish('Completed');
            controller.enqueue(encoder.encode('event: complete\ndata: {}\n\n'));
            controller.close();
          } else controller.enqueue(encode(item.value));
        } catch (error) {
          lifecycle.finish('Failed');
          try {
            const failure = mapError(error);
            controller.enqueue(
              encoder.encode(
                `event: error\ndata: ${JSON.stringify({ body: failure.body ?? null, status: failure.status })}\n\n`,
              ),
            );
            controller.close();
          } catch (encodingError) {
            controller.error(encodingError);
          } finally {
            await iterator.return?.().catch(() => undefined);
          }
        }
      },
    }),
    { headers: { 'Content-Type': 'text/event-stream' } },
  );
}

export function byteStreamResponse(
  source: AsyncIterable<Iterable<number>>,
): Response {
  return new Response(
    new ReadableStream<Uint8Array>({
      async start(controller) {
        for await (const item of source) {
          controller.enqueue(Uint8Array.from(item));
        }
        controller.close();
      },
    }),
    { headers: { 'Content-Type': 'application/octet-stream' } },
  );
}
