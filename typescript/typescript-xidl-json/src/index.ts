// biome-ignore-all lint/suspicious/noExplicitAny: JSON serialization library needs to handle arbitrary object structures.
import { z } from 'zod';

export interface XidlJsonMeta {
  name?: string;
  flatten?: boolean;
  ignore?: boolean;
  omitempty?: boolean;
}

const metaMap = new WeakMap<z.ZodTypeAny, XidlJsonMeta>();

export function getMeta(schema: z.ZodTypeAny): XidlJsonMeta | undefined {
  let current = schema;
  while (current) {
    const meta = metaMap.get(current);
    if (meta) return meta;

    if ('unwrap' in current && typeof (current as any).unwrap === 'function') {
      current = (current as any).unwrap();
    } else if ('_def' in current && (current as any)._def.innerType) {
      current = (current as any)._def.innerType;
    } else {
      break;
    }
  }
  return undefined;
}

export function setMeta<T extends z.ZodTypeAny>(
  schema: T,
  meta: XidlJsonMeta,
): T {
  metaMap.set(schema, meta);
  return schema;
}

// Convenient alias for setMeta
export function xjson<T extends z.ZodTypeAny>(
  schema: T,
  meta: XidlJsonMeta,
): T {
  return setMeta(schema, meta);
}

function unwrapSchema(schema: z.ZodTypeAny): z.ZodTypeAny {
  let current = schema;
  while (true) {
    if ('unwrap' in current && typeof (current as any).unwrap === 'function') {
      current = (current as any).unwrap();
    } else if ('_def' in current && (current as any)._def.innerType) {
      current = (current as any)._def.innerType;
    } else {
      break;
    }
  }
  return current;
}

function isZeroValue(val: any, schema: z.ZodTypeAny): boolean {
  if (val === null || val === undefined) return true;
  const unwrapped = unwrapSchema(schema);
  if (unwrapped instanceof z.ZodString) {
    return val === '';
  }
  if (unwrapped instanceof z.ZodNumber) {
    return val === 0;
  }
  if (unwrapped instanceof z.ZodBoolean) {
    return val === false;
  }
  if (unwrapped instanceof z.ZodArray) {
    return Array.isArray(val) && val.length === 0;
  }
  if (unwrapped instanceof z.ZodObject) {
    return typeof val === 'object' && Object.keys(val).length === 0;
  }
  return false;
}

export function serializeZodObject(obj: any, schema: z.ZodObject<any>): any {
  if (obj === null || typeof obj !== 'object') {
    return obj;
  }

  const result: Record<string, any> = {};
  const usedKeys = new Set<string>();

  // Collect all non-catch-all fields in first pass
  const catchAllFields: { key: string; fieldSchema: z.ZodTypeAny }[] = [];

  for (const key in schema.shape) {
    const fieldSchema = schema.shape[key];
    const meta = getMeta(fieldSchema) || {};
    if (meta.ignore) continue;

    const value = obj[key];

    // omitempty check
    if (meta.omitempty && isZeroValue(value, fieldSchema)) {
      continue;
    }

    const unwrapped = unwrapSchema(fieldSchema);

    if (meta.flatten) {
      if (unwrapped instanceof z.ZodObject) {
        if (value !== undefined && value !== null) {
          const innerSerialized = serializeZodObject(value, unwrapped);
          for (const innerKey in innerSerialized) {
            // Struct flatten depth conflict resolution:
            // "if two promoted fields share the same JSON key at the same depth, both are silently dropped."
            if (usedKeys.has(innerKey)) {
              delete result[innerKey];
            } else {
              result[innerKey] = innerSerialized[innerKey];
              usedKeys.add(innerKey);
            }
          }
        }
      } else {
        // It's a catch-all flatten field! Save for second pass.
        catchAllFields.push({ fieldSchema, key });
      }
    } else {
      const outKey = meta.name || key;
      // Normal field serialization
      if (value !== undefined) {
        result[outKey] = serialize(value, fieldSchema);
        usedKeys.add(outKey);
      }
    }
  }

  // Handle catch-all fields in second pass
  if (catchAllFields.length > 1) {
    throw new Error(
      'At most one catch-all flatten field is allowed per object.',
    );
  }

  if (catchAllFields.length === 1) {
    const { key } = catchAllFields[0];
    const value = obj[key];
    if (value !== undefined && value !== null) {
      if (typeof value !== 'object') {
        throw new Error('Catch-all flatten field value must be an object/map.');
      }
      for (const k in value) {
        // Named fields (in usedKeys) take priority and are skipped
        if (!usedKeys.has(k)) {
          result[k] = value[k];
        }
      }
    }
  }

  return result;
}

export function serialize(value: any, schema: z.ZodTypeAny): any {
  if (value === null || value === undefined) return value;
  const unwrapped = unwrapSchema(schema);

  if (unwrapped instanceof z.ZodObject) {
    return serializeZodObject(value, unwrapped);
  }
  if (unwrapped instanceof z.ZodArray) {
    if (!Array.isArray(value)) return value;
    const elementSchema = unwrapped.element;
    return value.map(item => serialize(item, elementSchema));
  }

  return value;
}

export function deserializeZodObject(
  jsonObj: any,
  schema: z.ZodObject<any>,
): any {
  if (jsonObj === null || typeof jsonObj !== 'object') {
    return jsonObj;
  }

  const result: Record<string, any> = {};
  const matchedKeys = new Set<string>();

  const catchAllFields: { key: string; fieldSchema: z.ZodTypeAny }[] = [];

  // First pass: extract all named fields
  for (const key in schema.shape) {
    const fieldSchema = schema.shape[key];
    const meta = getMeta(fieldSchema) || {};
    if (meta.ignore) continue;

    const unwrapped = unwrapSchema(fieldSchema);

    if (meta.flatten) {
      if (unwrapped instanceof z.ZodObject) {
        // Struct flatten
        const subResult = deserializeZodObject(jsonObj, unwrapped);
        result[key] = subResult;

        // Track which keys in jsonObj were matched by this sub-object
        const subExpectedKeys = getExpectedJsonKeys(unwrapped);
        for (const k of subExpectedKeys) {
          matchedKeys.add(k);
        }
      } else {
        // Catch-all flatten
        catchAllFields.push({ fieldSchema, key });
      }
    } else {
      const inKey = meta.name || key;
      const value = jsonObj[inKey];
      if (value !== undefined) {
        result[key] = deserialize(value, fieldSchema);
        matchedKeys.add(inKey);
      }
    }
  }

  // Second pass: extract catch-all if present
  if (catchAllFields.length > 1) {
    throw new Error(
      'At most one catch-all flatten field is allowed per object.',
    );
  }

  if (catchAllFields.length === 1) {
    const { key } = catchAllFields[0];
    const catchAllObj: Record<string, any> = {};
    for (const k in jsonObj) {
      if (!matchedKeys.has(k)) {
        catchAllObj[k] = jsonObj[k];
      }
    }
    result[key] = catchAllObj;
  }

  return result;
}

export function deserialize(value: any, schema: z.ZodTypeAny): any {
  if (value === null || value === undefined) return value;
  const unwrapped = unwrapSchema(schema);

  if (unwrapped instanceof z.ZodObject) {
    return deserializeZodObject(value, unwrapped);
  }
  if (unwrapped instanceof z.ZodArray) {
    if (!Array.isArray(value)) return value;
    const elementSchema = unwrapped.element;
    return value.map(item => deserialize(item, elementSchema));
  }

  return value;
}

function getExpectedJsonKeys(schema: z.ZodObject<any>): Set<string> {
  const keys = new Set<string>();
  for (const key in schema.shape) {
    const fieldSchema = schema.shape[key];
    const meta = getMeta(fieldSchema) || {};
    if (meta.ignore) continue;

    const unwrapped = unwrapSchema(fieldSchema);

    if (meta.flatten) {
      if (unwrapped instanceof z.ZodObject) {
        const subKeys = getExpectedJsonKeys(unwrapped);
        for (const k of subKeys) {
          keys.add(k);
        }
      }
    } else {
      keys.add(meta.name || key);
    }
  }
  return keys;
}
