import { serialize } from 'xidl-typescript-codec';
import type { ZodType } from 'zod';

import { encodeScalar, normalizeMime } from './scalar.js';
import { byteStreamResponse, sseResponse } from './stream.js';
import type {
  BodyField,
  HttpCodec,
  OperationDescriptor,
  ValueBinding,
} from './types.js';

export function encodeOperationResponse<TService>(
  operation: OperationDescriptor<TService>,
  result: unknown,
  codecs: Record<string, HttpCodec>,
): Response {
  if (operation.response.stream) {
    if (
      normalizeMime(operation.response.contentType) ===
      'application/octet-stream'
    ) {
      return byteStreamResponse(result as AsyncIterable<Iterable<number>>);
    }
    return sseResponse(
      result as AsyncIterable<unknown>,
      operation.response.streamSchema,
    );
  }

  const parsed = operation.response.schema?.parse(result) ?? result;
  const serialized = operation.response.schema
    ? serialize(parsed, operation.response.schema)
    : parsed;
  const record = toRecord(serialized);
  const headers = new Headers();
  writeBindings(headers, record, operation.response.headers, false);
  writeBindings(headers, record, operation.response.cookies, true);

  if (operation.response.bodyMode === 'none') {
    return new Response(null, { headers, status: 204 });
  }

  const body = selectBody(
    serialized,
    record,
    operation.response.bodyMode,
    operation.response.bodyFields,
  );
  const contentType = normalizeMime(operation.response.contentType);
  headers.set('Content-Type', contentType);
  return new Response(
    encodeBody(body, contentType, codecs, operation.response.schema),
    {
      headers,
      status: 200,
    },
  );
}

function toRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function selectBody(
  serialized: unknown,
  record: Record<string, unknown>,
  mode: 'object' | 'return',
  fields: BodyField[],
): unknown {
  if (mode === 'return') {
    const field = fields[0];
    if (!field) {
      return serialized;
    }
    const value = readField(record, field);
    return value === undefined ? serialized : value;
  }
  return Object.fromEntries(
    fields
      .map(field => [field.wireName, readField(record, field)])
      .filter(entry => entry[1] !== undefined),
  );
}

function writeBindings(
  headers: Headers,
  record: Record<string, unknown>,
  bindings: ValueBinding[],
  cookie: boolean,
): void {
  for (const binding of bindings) {
    const value = readField(record, binding);
    if (value === undefined || value === null) {
      continue;
    }
    const values = binding.multi && Array.isArray(value) ? value : [value];
    for (const item of values) {
      if (cookie) {
        headers.append(
          'Set-Cookie',
          `${binding.wireName}=${encodeURIComponent(encodeScalar(item))}`,
        );
      } else if (binding.multi) {
        headers.append(binding.wireName, encodeScalar(item));
      } else {
        headers.set(binding.wireName, encodeScalar(item));
      }
    }
  }
}

function readField(
  record: Record<string, unknown>,
  field: { key: string; wireName: string },
): unknown {
  return record[field.wireName] ?? record[field.key];
}

function encodeBody(
  value: unknown,
  contentType: string,
  codecs: Record<string, HttpCodec>,
  schema?: ZodType,
): BodyInit | null {
  const custom = codecs[contentType]?.encode;
  if (custom) {
    return custom(value, schema);
  }
  if (contentType === 'application/json' || contentType.endsWith('+json')) {
    return JSON.stringify(value);
  }
  if (contentType.startsWith('text/')) {
    return String(value ?? '');
  }
  throw new Error(`unsupported response content type: ${contentType}`);
}
