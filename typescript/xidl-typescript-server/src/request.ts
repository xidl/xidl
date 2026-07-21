import { deserialize } from 'xidl-typescript-codec';
import type { ZodType } from 'zod';

import { XidlServerError } from './error.js';
import {
  assertRequestContentType,
  normalizeMime,
  readRequestCookies,
} from './scalar.js';
import { byteRequestStream, ndjsonRequestStream } from './stream.js';
import type {
  HttpCodec,
  OperationDescriptor,
  RouteParams,
  ValueBinding,
} from './types.js';

export async function decodeOperationRequest<TService>(
  operation: OperationDescriptor<TService>,
  request: Request,
  params: RouteParams,
  codecs: Record<string, HttpCodec>,
): Promise<unknown> {
  if (operation.request.kind === 'stream') {
    assertRequestContentType(
      request.headers,
      operation.request.body.contentType,
    );
    if (
      normalizeMime(operation.request.body.contentType) ===
      'application/octet-stream'
    ) {
      return byteRequestStream(request);
    }
    return ndjsonRequestStream(request, operation.request.body.schema);
  }

  const payload: Record<string, unknown> = {};
  const url = new URL(request.url);
  for (const binding of operation.request.path) {
    const value = params[binding.parameter];
    const text = Array.isArray(value) ? value.join('/') : value;
    payload[binding.key] = text;
  }
  for (const binding of operation.request.query) {
    payload[binding.key] = readValues(
      url.searchParams.getAll(binding.wireName),
      binding,
    );
  }
  for (const binding of operation.request.headers) {
    const value = request.headers.get(binding.wireName);
    payload[binding.key] = readHeader(value, binding.multi);
  }
  const cookies = readRequestCookies(request.headers);
  for (const binding of operation.request.cookies) {
    payload[binding.key] = readValues(
      cookies.get(binding.wireName) ?? [],
      binding,
    );
  }

  if (operation.request.body.kind === 'value') {
    assertRequestContentType(
      request.headers,
      operation.request.body.contentType,
    );
    const body = await decodeRequestBody(
      request,
      operation.request.body.contentType,
      codecs,
      operation.request.body.schema,
    );
    applyBody(
      payload,
      body,
      operation.request.body.fields,
      operation.request.body.singleKey,
    );
  }

  if (operation.request.kind === 'none') {
    return undefined;
  }
  if (!operation.request.schema) {
    return payload;
  }
  return operation.request.schema.parse(
    deserialize(payload, operation.request.schema),
  );
}

function readValues(values: string[], binding: ValueBinding): unknown {
  if (values.length === 0) {
    return binding.multi ? [] : undefined;
  }
  return binding.multi ? values : (values[0] as string);
}

function readHeader(value: string | null, multi: boolean): unknown {
  if (value === null) {
    return multi ? [] : undefined;
  }
  if (!multi) {
    return value;
  }
  return value.split(',').map(item => item.trim());
}

async function decodeRequestBody(
  request: Request,
  contentType: string,
  codecs: Record<string, HttpCodec>,
  schema?: ZodType,
): Promise<unknown> {
  const mime = normalizeMime(
    contentType || request.headers.get('Content-Type') || 'application/json',
  );
  const custom = codecs[mime]?.decode;
  if (custom) {
    return custom(
      new Response(request.body, { headers: request.headers }),
      schema,
    );
  }
  if (mime === 'application/json' || mime.endsWith('+json')) {
    return request.json();
  }
  if (mime === 'application/x-www-form-urlencoded') {
    const form = new URLSearchParams(await request.text());
    return Object.fromEntries(
      [...new Set(form.keys())].map(key => {
        const values = form.getAll(key);
        return [key, values.length > 1 ? values : (values[0] ?? '')];
      }),
    );
  }
  if (mime.startsWith('text/')) {
    return request.text();
  }
  throw new XidlServerError(`unsupported request content type: ${mime}`, 415);
}

function applyBody(
  target: Record<string, unknown>,
  body: unknown,
  fields: Array<{ key: string; wireName: string }>,
  singleKey?: string,
): void {
  if (singleKey) {
    target[singleKey] = body;
    return;
  }
  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    return;
  }
  const record = body as Record<string, unknown>;
  for (const field of fields) {
    if (field.wireName in record) {
      target[field.key] = record[field.wireName];
    } else if (field.key in record) {
      target[field.key] = record[field.key];
    }
  }
}
