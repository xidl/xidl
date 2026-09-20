// biome-ignore-all lint/suspicious/noExplicitAny: JSON serialization library needs to handle arbitrary object structures.
/**
 * Minimal structural schema accepted by the codec.
 *
 * Only `parse` is required so schemas produced by any Zod 4.x copy
 * remain assignable even when the runtime and generated code resolve
 * different Zod versions with incompatible `_zod.version` literals.
 */
export interface XidlSchema {
  parse(value: unknown): unknown;
}

/** JSON wire metadata attached to a field schema. */
export interface XidlJsonMeta {
  name?: string;
  flatten?: boolean;
  ignore?: boolean;
  omitempty?: boolean;
}

const metaMap = new WeakMap<object, XidlJsonMeta>();

/** Read Zod metadata attached via {@link setMeta}. */
export function getMeta(schema: XidlSchema): XidlJsonMeta | undefined {
  let current: any = schema;
  while (current) {
    const meta = metaMap.get(current);
    if (meta) return meta;

    if (isArraySchema(current)) break;

    if (typeof current.unwrap === 'function') {
      current = current.unwrap();
    } else if (defOf(current)?.innerType) {
      current = defOf(current).innerType;
    } else {
      break;
    }
  }
  return undefined;
}

/** Attach JSON metadata to a schema without changing its type. */
export function setMeta<T extends XidlSchema>(
  schema: T,
  meta: XidlJsonMeta,
): T {
  metaMap.set(schema, meta);
  return schema;
}

/** Convenient alias for {@link setMeta}. */
export function xjson<T extends XidlSchema>(schema: T, meta: XidlJsonMeta): T {
  return setMeta(schema, meta);
}

function defOf(schema: any): any {
  return schema?._zod?.def ?? schema?._def ?? schema?.def;
}

function shapeOf(schema: any): Record<string, any> | undefined {
  const shape = schema?.shape;
  return shape && typeof shape === 'object' ? shape : undefined;
}

function elementOf(schema: any): any {
  return schema?.element ?? defOf(schema)?.element;
}

function defTypeOf(schema: any): string | undefined {
  return defOf(schema)?.type;
}

function isObjectSchema(schema: any): boolean {
  return defTypeOf(schema) === 'object' && shapeOf(schema) !== undefined;
}

function isArraySchema(schema: any): boolean {
  return defTypeOf(schema) === 'array';
}

function unwrapSchema(schema: XidlSchema): any {
  let current: any = schema;
  while (true) {
    if (isArraySchema(current)) break;

    if (typeof current?.unwrap === 'function') {
      current = current.unwrap();
    } else if (defOf(current)?.innerType) {
      current = defOf(current).innerType;
    } else {
      break;
    }
  }
  return current;
}

function isZeroValue(val: any, schema: XidlSchema): boolean {
  if (val === null || val === undefined) return true;
  const unwrapped = unwrapSchema(schema);
  const kind = defTypeOf(unwrapped);
  if (kind === 'string') {
    return val === '';
  }
  if (kind === 'number') {
    return val === 0;
  }
  if (kind === 'boolean') {
    return val === false;
  }
  if (isArraySchema(unwrapped)) {
    return Array.isArray(val) && val.length === 0;
  }
  if (isObjectSchema(unwrapped)) {
    return typeof val === 'object' && Object.keys(val).length === 0;
  }
  return false;
}

/** Serialize a Zod object using attached {@link XidlJsonMeta}. */
export function serializeZodObject(obj: any, schema: XidlSchema): any {
  if (obj === null || typeof obj !== 'object') {
    return obj;
  }

  const result: Record<string, any> = {};
  const usedKeys = new Set<string>();

  // Collect all non-catch-all fields in first pass
  const catchAllFields: { key: string; fieldSchema: XidlSchema }[] = [];

  const shape = shapeOf(schema) ?? {};
  for (const key in shape) {
    const fieldSchema = shape[key] as XidlSchema;
    const meta = getMeta(fieldSchema) || {};
    if (meta.ignore) continue;

    const value = obj[key];

    // omitempty check
    if (meta.omitempty && isZeroValue(value, fieldSchema)) {
      continue;
    }

    const unwrapped = unwrapSchema(fieldSchema);

    if (meta.flatten) {
      if (isObjectSchema(unwrapped)) {
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

/** Serialize a value using attached {@link XidlJsonMeta}. */
export function serialize(value: any, schema: XidlSchema): any {
  if (value === null || value === undefined) return value;
  const unwrapped = unwrapSchema(schema);

  if (isObjectSchema(unwrapped)) {
    return serializeZodObject(value, unwrapped);
  }
  if (isArraySchema(unwrapped)) {
    if (!Array.isArray(value)) return value;
    const elementSchema = elementOf(unwrapped) as XidlSchema;
    return value.map(item => serialize(item, elementSchema));
  }

  return value;
}

/** Deserialize JSON into a Zod object using attached {@link XidlJsonMeta}. */
export function deserializeZodObject(jsonObj: any, schema: XidlSchema): any {
  if (jsonObj === null || typeof jsonObj !== 'object') {
    return jsonObj;
  }

  const result: Record<string, any> = {};
  const matchedKeys = new Set<string>();

  const catchAllFields: { key: string; fieldSchema: XidlSchema }[] = [];

  // First pass: extract all named fields
  const shape = shapeOf(schema) ?? {};
  for (const key in shape) {
    const fieldSchema = shape[key] as XidlSchema;
    const meta = getMeta(fieldSchema) || {};
    if (meta.ignore) continue;

    const unwrapped = unwrapSchema(fieldSchema);

    if (meta.flatten) {
      if (isObjectSchema(unwrapped)) {
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

/** Deserialize a value using attached {@link XidlJsonMeta}. */
export function deserialize(value: any, schema: XidlSchema): any {
  if (value === null || value === undefined) return value;
  const unwrapped = unwrapSchema(schema);

  if (isObjectSchema(unwrapped)) {
    return deserializeZodObject(value, unwrapped);
  }
  if (isArraySchema(unwrapped)) {
    if (!Array.isArray(value)) return value;
    const elementSchema = elementOf(unwrapped) as XidlSchema;
    return value.map(item => deserialize(item, elementSchema));
  }

  return value;
}

function getExpectedJsonKeys(schema: XidlSchema): Set<string> {
  const keys = new Set<string>();
  const shape = shapeOf(schema) ?? {};
  for (const key in shape) {
    const fieldSchema = shape[key] as XidlSchema;
    const meta = getMeta(fieldSchema) || {};
    if (meta.ignore) continue;

    const unwrapped = unwrapSchema(fieldSchema);

    if (meta.flatten) {
      if (isObjectSchema(unwrapped)) {
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
