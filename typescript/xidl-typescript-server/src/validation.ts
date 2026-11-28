import { tryDeserialize, type XidlSchema } from 'xidl-typescript-codec';
import { RequestFailure, ResponseValidationError } from './error.ts';

/** Structural validation results also accept schemas from another Zod copy. */
export interface ServerSchema extends XidlSchema {
  safeParse(
    value: unknown,
  ):
    | { success: true; data: unknown }
    | { success: false; error: { issues: readonly unknown[] } };
}

export function parseRequest(value: unknown, schema: ServerSchema): unknown {
  const decoded = tryDeserialize(value, schema);
  if (!decoded.ok) {
    throw new RequestFailure(400, 'invalid request', decoded.issues);
  }
  const parsed = schema.safeParse(decoded.value);
  if (!parsed.success) {
    throw new RequestFailure(400, 'invalid request', parsed.error.issues);
  }
  return parsed.data;
}

export function parseResponse(value: unknown, schema?: ServerSchema): unknown {
  if (!schema) {
    return value;
  }
  const parsed = schema.safeParse(value);
  if (!parsed.success) {
    throw new ResponseValidationError(parsed.error.issues);
  }
  return parsed.data;
}
