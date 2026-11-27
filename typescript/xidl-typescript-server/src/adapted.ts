import { z } from 'zod';
import { RequestFailure } from './error.ts';
import { createRouter } from './router.ts';
import type {
  Awaitable,
  OperationDescriptor,
  ServerOptions,
  WebHandler,
} from './types.ts';
import {
  createVersionedRouter,
  type VersionEntry,
  type VersionedRouterOptions,
} from './versioned.ts';

interface AdapterValidationError {
  kind: 'Validation';
  version: string;
  operation: string;
  issues: readonly unknown[];
}

interface AdapterProjectionFailure {
  kind?: never;
  version: string;
  direction: 'upcast' | 'downcast';
  typePath: string;
  wireName?: string;
  reason: string;
}

/** Structural contract matching the generated dispatch and projection union. */
export type AdapterDispatchOutcome =
  | {
      dispatched: false;
      error: AdapterValidationError | AdapterProjectionFailure;
    }
  | {
      dispatched: true;
      projected: false;
      error: AdapterProjectionFailure;
      head: unknown;
    }
  | {
      dispatched: true;
      projected: true;
      value: { ok: true; value: unknown } | { ok: false; error: unknown };
    };

/** A conversion refusal retains the generated projection details and available Head outcome. */
export class VersionProjectionError extends Error {
  constructor(
    readonly projection: AdapterProjectionFailure,
    readonly head?: unknown,
  ) {
    super(projection.reason);
    this.name = 'VersionProjectionError';
  }
}

/** One operation dispatcher bound to its Head handler and adapter hooks. */
export interface AdapterOperationBinding<TVersionId extends string = string> {
  dispatch: (
    version: TVersionId,
    raw: Record<string, unknown>,
  ) => Awaitable<AdapterDispatchOutcome>;
}

export type AdaptedRouterOptions = ServerOptions;

export type OperationTable<TService> =
  | readonly OperationDescriptor<TService>[]
  | Readonly<Record<string, OperationDescriptor<TService>>>;

export interface VersionedAdapterEntry<TVersionId extends string = string> {
  id: TVersionId;
  value: string;
  // biome-ignore lint/suspicious/noExplicitAny: each version has its own generated service shape
  operations: OperationTable<any>;
}

export interface VersionedAdapterRouterOptions<
  TVersionId extends string = string,
> extends Omit<VersionedRouterOptions<TVersionId>, 'versions'> {
  versions: readonly VersionedAdapterEntry<TVersionId>[];
  bindings: Readonly<
    Record<string, AdapterOperationBinding<NoInfer<TVersionId>>>
  >;
  routerOptions?: AdaptedRouterOptions;
}

/**
 * Bridges a generated per-version operation table to generated adapter dispatchers.
 */
export function createAdaptedRouter<TVersionId extends string, TService>(
  version: TVersionId,
  operations: OperationTable<TService>,
  bindings: Readonly<
    Partial<Record<keyof TService, AdapterOperationBinding<TVersionId>>>
  >,
  options: AdaptedRouterOptions = {},
): WebHandler {
  const list = Array.isArray(operations)
    ? operations
    : Object.values(operations);
  const service = {} as Record<PropertyKey, (...args: unknown[]) => unknown>;

  for (const operation of list) {
    const handlerKey = operation.handler as keyof TService;
    const binding = bindings[handlerKey];
    if (!binding) {
      continue;
    }
    service[handlerKey] = async (...args: unknown[]) => {
      const raw: Record<string, unknown> = {};
      const argKeys = operation.request.args ?? [];
      for (let index = 0; index < argKeys.length; index += 1) {
        const key = argKeys[index];
        if (key !== undefined && args[index] !== undefined) {
          raw[key] = args[index];
        }
      }
      const outcome = await binding.dispatch(version, raw);
      return unwrapDispatchOutcome(outcome);
    };
  }

  const adaptedOperations = list.map(operation => {
    const handlerKey = operation.handler as keyof TService;
    return bindings[handlerKey]
      ? preserveWireBodyForAdapter(operation)
      : operation;
  });

  return createRouter(adaptedOperations, service as TService, options);
}

function preserveWireBodyForAdapter<TService>(
  operation: OperationDescriptor<TService>,
): OperationDescriptor<TService> {
  if (
    operation.request.kind !== 'object' ||
    operation.request.body.kind !== 'value'
  ) {
    return operation;
  }
  const bodyKeys = new Set<string>();
  if (operation.request.body.singleKey) {
    bodyKeys.add(operation.request.body.singleKey);
  }
  for (const field of operation.request.body.fields) {
    bodyKeys.add(field.key);
  }
  let schema: OperationDescriptor<TService>['request']['schema'];
  if (operation.request.schema instanceof z.ZodObject) {
    const shape: Record<string, z.ZodTypeAny> = {
      ...operation.request.schema.shape,
    };
    for (const key of bodyKeys) {
      shape[key] = z.unknown();
    }
    schema = z.object(shape);
  }
  return {
    ...operation,
    request: {
      ...operation.request,
      body: {
        ...operation.request.body,
        schema: undefined,
      },
      schema,
    },
  };
}

/**
 * Mounts every version of a contract through `createAdaptedRouter` in one handler.
 */
export function createVersionedAdapterRouter<TVersionId extends string>(
  options: VersionedAdapterRouterOptions<TVersionId>,
): WebHandler {
  const versions: VersionEntry<TVersionId>[] = options.versions.map(entry => ({
    handler: createAdaptedRouter(
      entry.id,
      entry.operations,
      options.bindings,
      options.routerOptions,
    ),
    id: entry.id,
    value: entry.value,
  }));
  return createVersionedRouter({
    defaultVersion: options.defaultVersion,
    onVersionError: options.onVersionError,
    parameter: options.parameter,
    retired: options.retired,
    versions,
  });
}

function unwrapDispatchOutcome(outcome: AdapterDispatchOutcome): unknown {
  if (!outcome.dispatched) {
    if (outcome.error.kind === 'Validation') {
      throw new RequestFailure(
        400,
        'request validation failed',
        outcome.error.issues,
      );
    }
    throw new VersionProjectionError(outcome.error);
  }
  if (!outcome.projected) {
    throw new VersionProjectionError(outcome.error, outcome.head);
  }
  if (outcome.value.ok) {
    return outcome.value.value;
  }
  const error = outcome.value.error;
  if (isFrameworkError(error)) {
    throw error.error;
  }
  throw error;
}

function isFrameworkError(
  error: unknown,
): error is { kind: 'Framework'; error: unknown } {
  return (
    typeof error === 'object' &&
    error !== null &&
    (error as { kind?: unknown }).kind === 'Framework' &&
    'error' in error
  );
}
