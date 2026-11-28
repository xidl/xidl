// biome-ignore-all lint/suspicious/noExplicitAny: JSON serialization library needs to handle arbitrary object structures.
import {
  defOf,
  defTypeOf,
  elementOf,
  isArraySchema,
  isObjectSchema,
  shapeOf,
  unwrapSchema,
} from './schema-structure.js';
import {
  captureWireDecode,
  decodeRequestUnion,
  deserializeUnion,
  type UnionDecoder,
  type WireDecode,
  type WireDecodeResult,
} from './union-decode.js';

export {
  decodeStreamFrame,
  SseDecoder,
  type StreamFailure,
  type StreamFrame,
  StreamInterruptedError,
  StreamLifecycle,
  type StreamOutcome,
} from './stream.js';
export type { WireDecodeResult } from './union-decode.js';
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

// Metadata belongs to the schema, including when runtimes load another codec copy.
const metadataKey = Symbol.for('xidl.json.metadata');

/** Read Zod metadata attached via {@link setMeta}. */
export function getMeta(schema: XidlSchema): XidlJsonMeta | undefined {
  let current: any = schema;
  while (current) {
    const meta: XidlJsonMeta | undefined = current[metadataKey];
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

/** Attach field metadata to a cloned Zod schema, preserving shared definitions. */
export function setMeta<T extends XidlSchema & { clone(): T }>(
  schema: T,
  meta: XidlJsonMeta,
): T {
  const annotated = schema.clone();
  Object.defineProperty(annotated, metadataKey, {
    configurable: true,
    value: meta,
  });
  return annotated;
}

/** Convenient alias for {@link setMeta}. */
export function xjson<T extends XidlSchema & { clone(): T }>(
  schema: T,
  meta: XidlJsonMeta,
): T {
  return setMeta(schema, meta);
}

function setJsonProperty(
  target: Record<string, any>,
  key: string,
  value: any,
): void {
  Object.defineProperty(target, key, {
    configurable: true,
    enumerable: true,
    value,
    writable: true,
  });
}

function transformRecordValues(
  value: any,
  schema: any,
  convert: (value: any, schema: XidlSchema) => any,
): any {
  if (typeof value !== 'object' || Array.isArray(value)) return value;
  const valueSchema = schema.valueType ?? defOf(schema)?.valueType;
  return Object.fromEntries(
    Object.entries(value).map(([key, item]) => [
      key,
      convert(item, valueSchema),
    ]),
  );
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

    const value = Object.hasOwn(obj, key) ? obj[key] : undefined;

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
              setJsonProperty(result, innerKey, innerSerialized[innerKey]);
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
        setJsonProperty(result, outKey, serialize(value, fieldSchema));
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
    const { key, fieldSchema } = catchAllFields[0];
    const value = Object.hasOwn(obj, key) ? obj[key] : undefined;
    if (value !== undefined && value !== null) {
      if (typeof value !== 'object') {
        throw new Error('Catch-all flatten field value must be an object/map.');
      }
      for (const [key, item] of Object.entries(serialize(value, fieldSchema))) {
        // Named fields (in usedKeys) take priority and are skipped.
        if (!usedKeys.has(key)) setJsonProperty(result, key, item);
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
  if (defTypeOf(unwrapped) === 'record') {
    return transformRecordValues(value, unwrapped, serialize);
  }
  if (defTypeOf(unwrapped) === 'union') {
    for (const option of defOf(unwrapped).options) {
      if (option.safeParse(value).success) return serialize(value, option);
    }
  }

  return value;
}

/** Deserialize JSON into a Zod object using attached {@link XidlJsonMeta}. */
export function deserializeZodObject(jsonObj: any, schema: XidlSchema): any {
  return decodeZodObject(jsonObj, schema, deserialize);
}

function decodeZodObject(
  jsonObj: any,
  schema: XidlSchema,
  decode: WireDecode,
): any {
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
        const subResult = decodeZodObject(jsonObj, unwrapped, decode);
        setJsonProperty(result, key, subResult);

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
      const value = Object.hasOwn(jsonObj, inKey) ? jsonObj[inKey] : undefined;
      if (value !== undefined) {
        setJsonProperty(result, key, decode(value, fieldSchema));
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
    const { key, fieldSchema } = catchAllFields[0];
    const catchAllObj: Record<string, any> = {};
    for (const [wireKey, value] of Object.entries(jsonObj)) {
      if (!matchedKeys.has(wireKey))
        setJsonProperty(catchAllObj, wireKey, value);
    }
    setJsonProperty(result, key, decode(catchAllObj, fieldSchema));
  }

  return result;
}

/** Deserialize a value using attached {@link XidlJsonMeta}. */
export function deserialize(value: any, schema: XidlSchema): any {
  return decodeValue(value, schema, deserialize, deserializeUnion);
}

/** Decode a wire value, reporting union validation rejections and propagating exceptions. */
export function tryDeserialize(
  value: unknown,
  schema: XidlSchema,
): WireDecodeResult {
  const decode: WireDecode = (input, inputSchema) =>
    decodeValue(input, inputSchema, decode, decodeRequestUnion);
  return captureWireDecode(() => decode(value, schema));
}

function decodeValue(
  value: any,
  schema: XidlSchema,
  decode: WireDecode,
  decodeUnion: UnionDecoder,
): any {
  if (value === null || value === undefined) return value;
  const unwrapped = unwrapSchema(schema);

  if (isObjectSchema(unwrapped)) {
    return decodeZodObject(value, unwrapped, decode);
  }
  if (isArraySchema(unwrapped)) {
    if (!Array.isArray(value)) return value;
    const elementSchema = elementOf(unwrapped) as XidlSchema;
    return value.map(item => decode(item, elementSchema));
  }
  if (defTypeOf(unwrapped) === 'record') {
    return transformRecordValues(value, unwrapped, decode);
  }
  if (defTypeOf(unwrapped) === 'union') {
    return decodeUnion(value, defOf(unwrapped).options, decode);
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
