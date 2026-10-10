import { deserialize, type XidlSchema } from 'xidl-typescript-codec';

import { XidlClientError } from './error.ts';
import { normalizeMime, parseScalar } from './scalar.ts';
import type { HttpCodec, ResponseValueSpec } from './types.ts';

export async function decodeResponseBody<T>(
  resp: Response,
  contentType: string,
  codecs: Record<string, HttpCodec>,
  schema?: XidlSchema,
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
  schema?: XidlSchema,
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
  bodyFields: Array<{ key: string; wireName: string }>,
  headerSpecs: ResponseValueSpec[],
  cookieSpecs: ResponseValueSpec[],
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  if (bodyMode === 'return' && body !== undefined && bodyFields[0]) {
    out[bodyFields[0].key] = body;
  } else if (bodyMode === 'object' && body && typeof body === 'object') {
    const record = body as Record<string, unknown>;
    for (const field of bodyFields) {
      out[field.key] = record[field.wireName];
    }
  }
  for (const spec of headerSpecs) {
    const value = readResponseHeader(
      resp.headers,
      spec.name,
      spec.isMulti,
      spec.decode,
    );
    if (value !== undefined) {
      out[spec.key] = value;
    }
  }
  const cookies = readResponseCookies(resp.headers, value => value);
  for (const spec of cookieSpecs) {
    const value = cookies.get(spec.name);
    if (value !== undefined) {
      const decoded = value.map(item =>
        (spec.decode ?? parseScalar)(item as string),
      );
      out[spec.key] = spec.isMulti ? decoded : decoded[0];
    }
  }
  return out;
}

export function readResponseHeader(
  headers: Headers,
  name: string,
  isMulti: boolean,
  decode: (value: string) => unknown = parseScalar,
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
      .map(decode);
  }
  return decode(value);
}

export function readResponseCookies(
  headers: Headers,
  decode: (value: string) => unknown = parseScalar,
): Map<string, unknown[]> {
  const out = new Map<string, unknown[]>();
  const raw =
    typeof (headers as Headers & { getSetCookie?: () => string[] })
      .getSetCookie === 'function'
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
    current.push(decode(value));
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
