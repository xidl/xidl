import { serialize } from 'xidl-typescript-codec';
import type { z } from 'zod';

import { XidlClientError } from './error.ts';
import { encodeScalar, normalizeMime } from './scalar.ts';
import type { ClientAuth, HttpCodec, SecurityRequirement } from './types.ts';

export function appendParam(
  query: URLSearchParams,
  key: string,
  value: unknown,
  optional: boolean,
  isMulti: boolean,
): void {
  if (value === undefined || value === null) {
    if (optional) {
      return;
    }
    query.append(key, '');
    return;
  }
  if (isMulti && Array.isArray(value)) {
    for (const item of value) {
      query.append(key, encodeScalar(item));
    }
    return;
  }
  query.append(key, encodeScalar(value));
}

export function appendHeader(
  headers: Headers,
  key: string,
  value: unknown,
  optional: boolean,
  isMulti: boolean,
): void {
  if (value === undefined || value === null) {
    return;
  }
  if (isMulti && Array.isArray(value)) {
    for (const item of value) {
      headers.append(key, encodeScalar(item));
    }
    return;
  }
  headers.set(key, encodeScalar(value));
}

export function appendCookie(
  cookies: string[],
  key: string,
  value: unknown,
  optional: boolean,
  isMulti: boolean,
): void {
  if (value === undefined || value === null) {
    return;
  }
  if (isMulti && Array.isArray(value)) {
    for (const item of value) {
      cookies.push(`${key}=${encodeURIComponent(encodeScalar(item))}`);
    }
    return;
  }
  cookies.push(`${key}=${encodeURIComponent(encodeScalar(value))}`);
}

export function applyCookies(headers: Headers, cookies: string[]): void {
  if (cookies.length === 0) {
    return;
  }
  const joined = cookies.join('; ');
  const current = headers.get('Cookie');
  headers.set('Cookie', current ? `${current}; ${joined}` : joined);
}

export function applyClientAuth(
  path: string,
  query: URLSearchParams,
  headers: Headers,
  auth: ClientAuth | undefined,
  requirements: SecurityRequirement[],
): void {
  if (!auth) {
    return;
  }
  const match = requirements.find(item => item.kind === auth.kind);
  if (!match) {
    return;
  }
  switch (auth.kind) {
    case 'basic':
      headers.set(
        'Authorization',
        `Basic ${btoa(`${auth.username}:${auth.password}`)}`,
      );
      break;
    case 'bearer':
      headers.set('Authorization', `Bearer ${auth.token}`);
      break;
    case 'api_key': {
      const name = auth.name ?? match.name ?? 'x-api-key';
      const location = auth.location ?? match.location ?? 'header';
      if (location === 'query') {
        query.set(name, auth.value);
      } else if (location === 'cookie') {
        const current = headers.get('Cookie');
        const next = `${name}=${encodeURIComponent(auth.value)}`;
        headers.set('Cookie', current ? `${current}; ${next}` : next);
      } else {
        headers.set(name, auth.value);
      }
      break;
    }
  }
  void path;
}

export function encodeRequestBody(
  value: unknown,
  contentType: string,
  codecs: Record<string, HttpCodec>,
  schema?: z.ZodTypeAny,
): BodyInit | null {
  const mime = normalizeMime(contentType);
  const custom = codecs[mime]?.encode;
  if (custom) {
    return custom(value, schema);
  }
  if (mime === 'application/json' || mime.endsWith('+json')) {
    const serialized = schema ? serialize(value, schema) : value;
    return JSON.stringify(serialized);
  }
  if (mime === 'application/x-www-form-urlencoded') {
    const query = new URLSearchParams();
    if (value && typeof value === 'object') {
      for (const [key, entry] of Object.entries(
        value as Record<string, unknown>,
      )) {
        appendParam(query, key, entry, true, Array.isArray(entry));
      }
    }
    return query;
  }
  throw new XidlClientError(
    `unsupported request content type: ${mime}`,
    500,
    500,
  );
}
