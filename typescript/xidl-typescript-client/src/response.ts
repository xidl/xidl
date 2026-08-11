import { deserialize } from 'xidl-typescript-codec';
import type { z } from 'zod';

import { XidlClientError } from './error.js';
import { normalizeMime, parseScalar } from './scalar.js';
import type { HttpCodec, ResponseValueSpec } from './types.js';

export async function decodeResponseBody<T>(
  resp: Response,
  contentType: string,
  codecs: Record<string, HttpCodec>,
  schema?: z.ZodTypeAny,
): Promise<T> {
  const mime = normalizeMime(
    contentType || resp.headers.get('Content-Type') || 'application/json',
  );
  const custom = codecs[mime]?.decode;
  if (custom) {
    return custom<T>(resp, schema);
  }
  if (mime === 'application/json' || mime.endsWith('+json')) {
    const data = await resp.json();
    return (schema ? deserialize(data, schema) : data) as T;
  }
  if (mime.startsWith('text/')) {
    return (await resp.text()) as T;
  }
  throw new XidlClientError(
    `unsupported response content type: ${mime}`,
    500,
    resp.status,
  );
}

export async function decodeOptionalResponseBody(
  resp: Response,
  contentType: string,
  codecs: Record<string, HttpCodec>,
  schema?: z.ZodTypeAny,
): Promise<unknown> {
  if (resp.status === 204 || resp.status === 205 || !resp.body) {
    return undefined;
  }
  const length = resp.headers.get('Content-Length');
  if (length === '0') {
    return undefined;
  }
  return decodeResponseBody(resp, contentType, codecs, schema);
}

export function buildResponsePayload(
  body: unknown,
  resp: Response,
  bodyMode: string,
  headerSpecs: ResponseValueSpec[],
  cookieSpecs: ResponseValueSpec[],
): Record<string, unknown> {
  const out: Record<string, unknown> =
    bodyMode === 'object' && body && typeof body === 'object'
      ? { ...(body as Record<string, unknown>) }
      : {};
  if (bodyMode === 'return' && body !== undefined) {
    out.return = body;
  }
  for (const spec of headerSpecs) {
    const value = readResponseHeader(resp.headers, spec.name, spec.isMulti);
    if (value !== undefined) {
      out[spec.key] = value;
    }
  }
  const cookies = readResponseCookies(resp.headers);
  for (const spec of cookieSpecs) {
    const value = cookies.get(spec.name);
    if (value !== undefined) {
      out[spec.key] = spec.isMulti ? value : value[0];
    }
  }
  return out;
}

export function readResponseHeader(
  headers: Headers,
  name: string,
  isMulti: boolean,
): unknown {
  const value = headers.get(name);
  if (value === null) {
    return undefined;
  }
  if (isMulti) {
    return value
      .split(',')
      .map(item => item.trim())
      .filter(item => item.length > 0)
      .map(parseScalar);
  }
  return parseScalar(value);
}

export function readResponseCookies(headers: Headers): Map<string, string[]> {
  const out = new Map<string, string[]>();
  const raw =
    typeof (headers as Headers & { getSetCookie?: () => string[] }).getSetCookie ===
    'function'
      ? (headers as Headers & { getSetCookie: () => string[] }).getSetCookie()
      : headers.get('Set-Cookie')
        ? [headers.get('Set-Cookie') as string]
        : [];
  for (const line of raw) {
    const pair = line.split(';')[0];
    if (!pair) {
      continue;
    }
    const idx = pair.indexOf('=');
    if (idx < 0) {
      continue;
    }
    const name = pair.slice(0, idx).trim();
    const value = decodeURIComponent(pair.slice(idx + 1));
    const current = out.get(name) ?? [];
    current.push(parseScalar(value) as string);
    out.set(name, current);
  }
  return out;
}

export async function parseXidlError(resp: Response): Promise<XidlClientError> {
  const status = resp.status;
  try {
    const body = await resp.json();
    if (body && typeof body.code === 'number' && typeof body.msg === 'string') {
      return new XidlClientError(body.msg, body.code, status);
    }
  } catch {
    // ignored
  }
  return new XidlClientError(`http error: ${status}`, status, status);
}
