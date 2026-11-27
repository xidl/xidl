import {
  decodeStreamFrame,
  type StreamFrame,
  StreamLifecycle,
  serialize,
} from 'xidl-typescript-codec';
import {
  declaredErrorResponse,
  RequestFailure,
  RequestStreamFailure,
} from './error.ts';
import type { TypedErrorResponse } from './types.ts';
import {
  parseRequest,
  parseResponse,
  type ServerSchema,
} from './validation.ts';

function readFrame(line: string): StreamFrame {
  try {
    return decodeStreamFrame(JSON.parse(line));
  } catch (error) {
    throw new RequestFailure(400, `invalid NDJSON request: ${String(error)}`);
  }
}

export async function* ndjsonRequestStream<T>(
  request: Request,
  schema?: ServerSchema,
): AsyncIterable<T> {
  if (!request.body) {
    throw new RequestStreamFailure(
      new RequestFailure(400, 'NDJSON request has no body'),
    );
  }
  const reader = request.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  try {
    while (true) {
      const next = await reader.read();
      if (next.done) {
        throw new RequestFailure(
          400,
          'NDJSON request ended without a terminal frame',
        );
      }
      buffer += decoder.decode(next.value, { stream: true });
      let end = buffer.indexOf('\n');
      while (end >= 0) {
        const line = buffer.slice(0, end).trim();
        buffer = buffer.slice(end + 1);
        if (line) {
          const frame = readFrame(line);
          if (frame.t === 'complete') return;
          if (frame.t === 'error') {
            throw new RequestFailure(
              frame.error.status >= 400 ? frame.error.status : 400,
              'NDJSON producer failed',
            );
          }
          if (frame.t === 'next') {
            yield (schema ? parseRequest(frame.data, schema) : frame.data) as T;
          }
        }
        end = buffer.indexOf('\n');
      }
    }
  } catch (error) {
    if (error instanceof RequestFailure) {
      throw new RequestStreamFailure(error);
    }
    throw error;
  } finally {
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
  schema?: ServerSchema,
  matchError?: (error: unknown) => TypedErrorResponse | undefined,
): Promise<Response> {
  const iterator = source[Symbol.asyncIterator]();
  const encoder = new TextEncoder();
  const lifecycle = new StreamLifecycle();
  const complete = encoder.encode('event: complete\ndata: {}\n\n');

  const closeAfterFailure = async () => {
    try {
      await iterator.return?.();
    } catch {
      // An exception already determined the stream's outcome.
    }
  };
  const encode = (value: T): Uint8Array => {
    const wire = schema
      ? serialize(parseResponse(value, schema), schema)
      : value;
    const json = JSON.stringify(wire);
    if (json === undefined) {
      throw new Error('SSE item has no JSON representation');
    }
    return encoder.encode(`event: next\ndata: ${json}\n\n`);
  };

  let first: IteratorResult<T>;
  try {
    first = await iterator.next();
  } catch (error) {
    await closeAfterFailure();
    const declared = RequestFailure.from(error)
      ? undefined
      : matchError?.(error);
    if (declared) {
      return declaredErrorResponse(declared);
    }
    throw error;
  }

  let pending: Uint8Array | undefined;
  try {
    if (first.done) {
      lifecycle.finish('Completed');
      pending = complete;
    } else {
      pending = encode(first.value);
    }
  } catch (error) {
    await closeAfterFailure();
    throw error;
  }

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
        let item: IteratorResult<T>;
        try {
          item = await iterator.next();
        } catch (error) {
          lifecycle.finish('Failed');
          await closeAfterFailure();
          try {
            const declared = RequestFailure.from(error)
              ? undefined
              : matchError?.(error);
            if (!declared) {
              controller.error(error);
              return;
            }
            const payload = JSON.stringify({
              body: declared.body ?? null,
              status: declared.status,
            });
            controller.enqueue(
              encoder.encode(`event: error\ndata: ${payload}\n\n`),
            );
            controller.close();
          } catch (encodingError) {
            controller.error(encodingError);
          }
          return;
        }
        if (item.done) {
          lifecycle.finish('Completed');
          controller.enqueue(complete);
          controller.close();
          return;
        }
        try {
          controller.enqueue(encode(item.value));
        } catch (error) {
          lifecycle.finish('Failed');
          await closeAfterFailure();
          controller.error(error);
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
