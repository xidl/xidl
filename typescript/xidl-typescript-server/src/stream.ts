import { deserialize, serialize } from 'xidl-typescript-codec';
import type { ZodType } from 'zod';

export async function* ndjsonRequestStream<T>(
  request: Request,
  schema?: ZodType,
): AsyncIterable<T> {
  if (!request.body) {
    return;
  }
  const reader = request.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  while (true) {
    const next = await reader.read();
    if (next.done) {
      break;
    }
    buffer += decoder.decode(next.value, { stream: true });
    let separator = buffer.indexOf('\n');
    while (separator >= 0) {
      const line = buffer.slice(0, separator).trim();
      buffer = buffer.slice(separator + 1);
      if (line) {
        const value = JSON.parse(line);
        yield (schema ? deserialize(value, schema) : value) as T;
      }
      separator = buffer.indexOf('\n');
    }
  }
  const line = buffer.trim();
  if (line) {
    const value = JSON.parse(line);
    yield (schema ? deserialize(value, schema) : value) as T;
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

export function sseResponse<T>(
  source: AsyncIterable<T>,
  schema?: ZodType,
): Response {
  const encoder = new TextEncoder();
  return new Response(
    new ReadableStream<Uint8Array>({
      async start(controller) {
        for await (const item of source) {
          const value = schema ? serialize(item, schema) : item;
          controller.enqueue(
            encoder.encode(`event: next\ndata: ${JSON.stringify(value)}\n\n`),
          );
        }
        controller.enqueue(encoder.encode('event: complete\ndata: done\n\n'));
        controller.close();
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
