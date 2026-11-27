import { deserialize, type XidlSchema } from 'xidl-typescript-codec';
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
  let decoded: unknown;
  try {
    decoded = deserialize(value, schema);
  } catch (error) {
    // Union decoding can throw a validation error from the schema's Zod copy.
    if (
      error &&
      typeof error === 'object' &&
      'name' in error &&
      (error.name === 'ZodError' || error.name === '$ZodError') &&
      'issues' in error &&
      Array.isArray(error.issues)
    ) {
      throw new RequestFailure(400, 'invalid request', error.issues);
    }
    throw error;
  }
  const parsed = schema.safeParse(decoded);
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
