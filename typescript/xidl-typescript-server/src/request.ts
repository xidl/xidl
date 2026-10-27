import { deserialize, type XidlSchema } from 'xidl-typescript-codec';

import { XidlServerError } from './error.ts';
import {
  assertRequestContentType,
  normalizeMime,
  readRequestCookies,
} from './scalar.ts';
import { byteRequestStream, ndjsonRequestStream } from './stream.ts';
import type {
  HttpCodec,
  OperationDescriptor,
  RequestValueBinding,
  RouteParams,
} from './types.ts';

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
      'query',
    );
  }
  for (const binding of operation.request.headers) {
    const value = request.headers.get(binding.wireName);
    payload[binding.key] = readHeader(value, binding);
  }
  const cookies = readRequestCookies(request.headers);
  for (const binding of operation.request.cookies) {
    payload[binding.key] = readValues(
      cookies.get(binding.wireName) ?? [],
      binding,
      'cookie',
    );
  }

  if (operation.request.body.kind === 'value') {
    if (
      normalizeMime(operation.request.body.contentType) ===
      'application/octet-stream'
    ) {
      // Raw byte payload: the declared Content-Type is the payload's own
      // media type and is not constrained; the bytes are the value.
      const bytes = new Uint8Array(await request.arrayBuffer());
      applyBody(
        payload,
        bytes,
        operation.request.body.fields,
        operation.request.body.singleKey,
      );
    } else {
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

function readValues(
  values: string[],
  binding: RequestValueBinding,
  source: 'query' | 'header' | 'cookie',
): unknown {
  if (values.length === 0) {
    return binding.multi && !binding.optional ? [] : undefined;
  }
  // Only query extraction rejects duplicates; the Rust target's header and
  // cookie maps keep the first occurrence for scalar bindings.
  if (source === 'query' && !binding.multi && values.length > 1) {
    throw new XidlServerError(
      400,
      `duplicate value for parameter ${binding.wireName}`,
    );
  }
  return binding.multi ? values : (values[0] as string);
}

function readHeader(
  value: string | null,
  binding: RequestValueBinding,
): unknown {
  if (value === null) {
    return binding.multi && !binding.optional ? [] : undefined;
  }
  if (!headerValueIsVisibleAscii(value)) {
    throw new XidlServerError(
      400,
      `header ${binding.wireName} contains non-ASCII bytes`,
    );
  }
  if (!binding.multi) {
    return value;
  }
  return value.split(',').map(item => item.trim());
}

// The Rust target extracts headers through HeaderValue::to_str, which only
// accepts visible ASCII plus space and tab; mirror that acceptance here.
function headerValueIsVisibleAscii(value: string): boolean {
  return /^[\t\x20-\x7E]*$/.test(value);
}

async function decodeRequestBody(
  request: Request,
  contentType: string,
  codecs: Record<string, HttpCodec>,
  schema?: XidlSchema,
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
  throw new XidlServerError(415, `unsupported request content type: ${mime}`);
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

/** Decode captured path parameters after a route and its operation are selected. */
export function decodeRouteParams(params: RouteParams): RouteParams {
  const decode = (value: string): string => {
    try {
      return decodeURIComponent(value);
    } catch {
      throw new XidlServerError(400, `invalid route segment: ${value}`);
    }
  };
  return Object.fromEntries(
    Object.entries(params).map(([name, value]) => [
      name,
      typeof value === 'string' ? decode(value) : value?.map(decode),
    ]),
  );
}
