import { z } from 'zod';
import { RequestFailure } from './error.ts';
import { createBoundRouter, createRouter } from './router.ts';
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

export type OperationTable<
  TService,
  THandler extends keyof TService = keyof TService,
> =
  | readonly OperationDescriptor<TService, THandler>[]
  | Readonly<Record<string, OperationDescriptor<TService, THandler>>>;

type AdapterService = Record<PropertyKey, (...args: unknown[]) => unknown>;

/** One version's interface operations, each paired with its Head dispatcher. */
export interface BoundInterface<TVersionId extends string = string> {
  readonly version: TVersionId;
  readonly operations: readonly {
    readonly operation: OperationDescriptor<AdapterService>;
    readonly binding: AdapterOperationBinding<TVersionId>;
  }[];
}

export interface VersionedAdapterEntry<TVersionId extends string = string> {
  id: TVersionId;
  value: string;
  interfaces: readonly BoundInterface<NoInfer<TVersionId>>[];
}

export interface VersionedAdapterRouterOptions<
  TVersionId extends string = string,
> extends Omit<VersionedRouterOptions<TVersionId>, 'versions'> {
  versions: readonly VersionedAdapterEntry<TVersionId>[];
  routerOptions?: AdaptedRouterOptions;
}

/** Mounts one operation table using a binding for each operation. */
export function createAdaptedRouter<
  TVersionId extends string,
  TService,
  THandler extends keyof TService,
>(
  version: TVersionId,
  operations: OperationTable<TService, THandler>,
  bindings: Readonly<
    Record<NoInfer<THandler>, AdapterOperationBinding<NoInfer<TVersionId>>>
  >,
  options: AdaptedRouterOptions = {},
): WebHandler {
  const list: readonly OperationDescriptor<TService, THandler>[] =
    Array.isArray(operations) ? operations : Object.values(operations);
  const group = {
    operations: list.map(operation => {
      const binding = bindings[operation.handler];
      if (!binding) {
        throw new Error(
          `missing adapter binding for '${String(operation.handler)}'`,
        );
      }
      return { binding, operation };
    }),
    version,
  };
  const adapted = attachInterface(group, 'allow');
  return createRouter(adapted.operations, adapted.service, options);
}

function attachInterface<TVersionId extends string>(
  group: BoundInterface<TVersionId>,
  duplicateHandlers: 'allow' | 'reject',
) {
  const service: AdapterService = Object.create(null);
  const operations = group.operations.map(({ operation, binding }) => {
    if (
      duplicateHandlers === 'reject' &&
      Object.hasOwn(service, operation.handler)
    ) {
      throw new Error(
        `duplicate adapter handler '${String(operation.handler)}' in an operation table`,
      );
    }
    service[operation.handler] = async (...args: unknown[]) => {
      const raw: Record<string, unknown> = {};
      const argKeys = operation.request.args ?? [];
      for (let index = 0; index < argKeys.length; index += 1) {
        const key = argKeys[index];
        if (key !== undefined && args[index] !== undefined) {
          raw[key] = args[index];
        }
      }
      return unwrapDispatchOutcome(await binding.dispatch(group.version, raw));
    };
    return preserveWireBodyForAdapter(operation);
  });
  return { operations, service };
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

/** Mounts all bound interfaces of each version behind one Web handler. */
export function createVersionedAdapterRouter<TVersionId extends string>(
  options: VersionedAdapterRouterOptions<TVersionId>,
): WebHandler {
  const versions: VersionEntry<TVersionId>[] = options.versions.map(entry => ({
    handler: createBoundRouter(
      entry.interfaces.flatMap(group => {
        if (group.version !== entry.id) {
          throw new Error(
            `cannot mount interface from version '${group.version}' on '${entry.id}'`,
          );
        }
        const { operations, service } = attachInterface(group, 'reject');
        return operations.map(operation => ({ operation, service }));
      }),
      options.routerOptions ?? {},
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
