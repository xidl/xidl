import { z } from 'zod';
import { XidlServerError } from './error.ts';
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
  type VersionLifecycle,
} from './versioned.ts';

/** Structural contract matching generated `DispatchOutcome` from `typescript-rest-adapters`. */
export interface AdapterDispatchOutcome {
  dispatched: boolean;
  projected?: boolean;
  error?: unknown;
  value?: { ok: true; value: unknown } | { ok: false; error: unknown };
}

/** One operation dispatcher bound to its Head handler and adapter hooks. */
export interface AdapterOperationBinding<TVersionId extends string = string> {
  dispatch: (
    version: TVersionId,
    raw: Record<string, unknown>,
  ) => Awaitable<AdapterDispatchOutcome>;
}

export interface AdaptedRouterOptions extends ServerOptions {
  /** Converts a dispatch failure (`!dispatched`) into a thrown error. */
  onDispatchError?: (error: unknown) => unknown;
  /** Converts a downcast refusal (`!projected`) into a thrown error. */
  onProjectionError?: (error: unknown) => unknown;
}

export type OperationTable<TService> =
  | readonly OperationDescriptor<TService>[]
  | Readonly<Record<string, OperationDescriptor<TService>>>;

export interface VersionedAdapterEntry<TVersionId extends string = string> {
  id: TVersionId;
  value: string;
  status?: VersionLifecycle;
  // biome-ignore lint/suspicious/noExplicitAny: each version has its own generated service shape
  operations?: OperationTable<any>;
}

export interface VersionedAdapterRouterOptions<
  TVersionId extends string = string,
> extends Omit<VersionedRouterOptions<TVersionId>, 'versions'> {
  versions: readonly VersionedAdapterEntry<TVersionId>[];
  bindings: Readonly<Record<string, AdapterOperationBinding<TVersionId>>>;
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
      return unwrapDispatchOutcome(outcome, options);
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
    handler:
      entry.status === 'retired' || !entry.operations
        ? undefined
        : createAdaptedRouter(
            entry.id,
            entry.operations,
            options.bindings,
            options.routerOptions,
          ),
    id: entry.id,
    status: entry.status,
    value: entry.value,
  }));
  return createVersionedRouter({
    defaultVersion: options.defaultVersion,
    echoHeader: options.echoHeader,
    onVersionError: options.onVersionError,
    parameter: options.parameter,
    resolveVersion: options.resolveVersion,
    versions,
  });
}

function unwrapDispatchOutcome(
  outcome: AdapterDispatchOutcome,
  options: AdaptedRouterOptions,
): unknown {
  if (!outcome.dispatched) {
    if (options.onDispatchError) {
      throw options.onDispatchError(outcome.error);
    }
    throw formatAdapterFailure(400, outcome.error, 'invalid versioned request');
  }
  if (!outcome.projected) {
    if (options.onProjectionError) {
      throw options.onProjectionError(outcome.error);
    }
    throw formatAdapterFailure(
      422,
      outcome.error,
      'cannot project response to requested version',
    );
  }
  if (!outcome.value) {
    throw new XidlServerError(500, 'adapter returned empty outcome');
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

function formatAdapterFailure(
  status: number,
  error: unknown,
  fallback: string,
): XidlServerError {
  if (error instanceof XidlServerError) {
    return error;
  }
  if (typeof error === 'object' && error !== null) {
    const reason = (error as { reason?: unknown }).reason;
    if (typeof reason === 'string' && reason.length > 0) {
      return new XidlServerError(status, reason);
    }
    const kind = (error as { kind?: unknown }).kind;
    if (kind === 'Validation') {
      return new XidlServerError(status, 'request validation failed');
    }
  }
  return new XidlServerError(status, fallback);
}
