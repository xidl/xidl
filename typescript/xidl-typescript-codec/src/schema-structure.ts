// biome-ignore-all lint/suspicious/noExplicitAny: Inspect schema definitions across Zod versions.
import type { XidlSchema } from './index.js';

export function defOf(schema: any): any {
  return schema?._zod?.def ?? schema?._def ?? schema?.def;
}

export function shapeOf(schema: any): Record<string, any> | undefined {
  const shape = schema?.shape;
  return shape && typeof shape === 'object' ? shape : undefined;
}

export function elementOf(schema: any): any {
  return schema?.element ?? defOf(schema)?.element;
}

export function defTypeOf(schema: any): string | undefined {
  return defOf(schema)?.type;
}

export function isObjectSchema(schema: any): boolean {
  return defTypeOf(schema) === 'object' && shapeOf(schema) !== undefined;
}

export function isArraySchema(schema: any): boolean {
  return defTypeOf(schema) === 'array';
}

export function unwrapSchema(schema: XidlSchema): any {
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
