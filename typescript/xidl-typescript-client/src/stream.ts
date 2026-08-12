import { deserialize, serialize } from 'xidl-typescript-codec';
import type { z } from 'zod';

import { XidlClientError } from './error.ts';
import { parseXidlError } from './response.ts';
import type { FetchLike } from './types.ts';

export async function* sseJsonStream<T>(
  fetchImpl: FetchLike,
  url: string,
  options: RequestInit,
  schema?: z.ZodTypeAny,
): AsyncIterable<T> {
  const resp = await fetchImpl(url, options);
  if (!resp.ok) {
    throw await parseXidlError(resp);
  }
  if (!resp.body) {
    throw new XidlClientError('sse response has no body', 500, resp.status);
  }
  const reader = resp.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  let event = 'message';
  let dataLines: string[] = [];
  const flush = (): { done: boolean; value?: T } | null => {
    if (event.length === 0 && dataLines.length === 0) {
      return null;
    }
    const payload = dataLines.join('\n');
    const current = event || 'message';
    event = 'message';
    dataLines = [];
    if (current === 'complete') {
      return { done: true };
    }
    if (current === 'next' || current === 'message') {
      const data = JSON.parse(payload);
      const value = schema ? (deserialize(data, schema) as T) : (data as T);
      return { done: false, value };
    }
    if (current === 'error') {
      throw new XidlClientError(payload || 'stream error', 500, resp.status);
    }
    return null;
  };

  while (true) {
    const next = await reader.read();
    if (next.done) {
      break;
    }
    buffer += decoder.decode(next.value, { stream: true });
    while (true) {
      const idx = buffer.indexOf('\n');
      if (idx < 0) {
        break;
      }
      const line = buffer.slice(0, idx).replace(/\r$/, '');
      buffer = buffer.slice(idx + 1);
      if (line.length === 0) {
        const flushed = flush();
        if (flushed?.done) {
          return;
        }
        if (flushed?.value !== undefined) {
          yield flushed.value;
        }
        continue;
      }
      if (line.startsWith('event:')) {
        event = line.slice('event:'.length).trim();
      } else if (line.startsWith('data:')) {
        dataLines.push(line.slice('data:'.length).trimStart());
      }
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
  schema?: z.ZodTypeAny,
): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  const iterator = source[Symbol.asyncIterator]();
  return new ReadableStream<Uint8Array>({
    async pull(controller) {
      const next = await iterator.next();
      if (next.done) {
        controller.close();
        return;
      }
      const serialized = schema ? serialize(next.value, schema) : next.value;
      controller.enqueue(encoder.encode(`${JSON.stringify(serialized)}\n`));
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
