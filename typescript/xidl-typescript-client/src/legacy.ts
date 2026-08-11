import { deserialize, serialize } from 'xidl-typescript-codec';
import type { z } from 'zod';

import { XidlClientError } from './error.js';
import { parseXidlError } from './response.js';
import type { FetchLike } from './types.js';

export async function* sseJsonStreamLegacy<T>(
  fetchImpl: FetchLike,
  url: string,
  options: RequestInit,
  schema?: z.ZodTypeAny,
): AsyncIterable<T> {
  const headers = new Headers(options.headers ?? {});
  if (!headers.has('Accept')) {
    headers.set('Accept', 'text/event-stream');
  }
  const resp = await fetchImpl(url, { ...options, headers });
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

  const flushEvent = (): { done: boolean; value?: T } | null => {
    if (event.length === 0 && dataLines.length === 0) {
      return null;
    }
    const payload = dataLines.join('\n');
    const currentEvent = event || 'message';
    event = 'message';
    dataLines = [];
    if (currentEvent === 'complete') {
      return { done: true };
    }
    if (currentEvent === 'error') {
      let msg = payload;
      let code = 500;
      try {
        const parsed = JSON.parse(payload);
        if (parsed && typeof parsed.msg === 'string') {
          msg = parsed.msg;
        }
        if (parsed && typeof parsed.code === 'number') {
          code = parsed.code;
        }
      } catch {
        // keep raw payload
      }
      throw new XidlClientError(msg || 'stream error', code, resp.status);
    }
    if (currentEvent === 'next' || currentEvent === 'message') {
      const parsed = JSON.parse(payload);
      const value = schema ? (deserialize(parsed, schema) as T) : (parsed as T);
      return { done: false, value };
    }
    return null;
  };

  while (true) {
    const { value, done } = await reader.read();
    if (done) {
      break;
    }
    buffer += decoder.decode(value, { stream: true });
    while (true) {
      const idx = buffer.indexOf('\n');
      if (idx < 0) {
        break;
      }
      const line = buffer.slice(0, idx).replace(/\r$/, '');
      buffer = buffer.slice(idx + 1);
      if (line.length === 0) {
        const flushed = flushEvent();
        if (flushed?.done) {
          return;
        }
        if (flushed?.value !== undefined) {
          yield flushed.value;
        }
        continue;
      }
      if (line.startsWith(':')) {
        continue;
      }
      if (line.startsWith('event:')) {
        event = line.slice('event:'.length).trim();
        continue;
      }
      if (line.startsWith('data:')) {
        dataLines.push(line.slice('data:'.length).trimStart());
      }
    }
  }
}

export function ndjsonBodyLegacy<T>(
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
    async cancel() {
      if (iterator.return) {
        await iterator.return();
      }
    },
  });
}
