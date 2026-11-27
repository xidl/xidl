import { executeOperation } from './execute.ts';
import { decodeRouteParams } from './request.ts';
import type {
  OperationDescriptor,
  RouteParams,
  ServerOptions,
  WebHandler,
} from './types.ts';

export interface BoundOperation<TService> {
  operation: OperationDescriptor<TService>;
  service: TService;
}

interface Route<TService> extends BoundOperation<TService> {
  path: string;
  segments: string[];
}

export function createRouter<TService>(
  operations: readonly OperationDescriptor<TService>[],
  service: TService,
  options: ServerOptions = {},
): WebHandler {
  const routes = expandRoutes(
    operations.map(operation => ({ operation, service })),
  );
  return handleRoutes(routes, options);
}

export function createBoundRouter<TService>(
  operations: readonly BoundOperation<TService>[],
  options: ServerOptions,
): WebHandler {
  const routes = expandRoutes(operations);
  const owners = new Map<string, Route<TService>>();
  for (const route of routes) {
    const shape = route.segments.map(segment => {
      const parameter = parseParameter(segment);
      return parameter ? [parameter.catchAll] : segment;
    });
    const key = JSON.stringify([route.operation.method, shape]);
    const owner = owners.get(key);
    if (
      owner &&
      (owner.service !== route.service || owner.operation !== route.operation)
    ) {
      const scope =
        owner.service === route.service
          ? 'within an operation table'
          : 'across operation tables';
      throw new Error(
        `conflicting ${route.operation.method} route ${route.path} ${scope}`,
      );
    }
    owners.set(key, route);
  }

  return handleRoutes(routes, options);
}

function expandRoutes<TService>(
  operations: readonly BoundOperation<TService>[],
): Route<TService>[] {
  return operations.flatMap(({ operation, service }) =>
    (operation.paths ?? [operation.path]).map(path => ({
      operation,
      path,
      segments: splitPath(path),
      service,
    })),
  );
}

function handleRoutes<TService>(
  routes: readonly Route<TService>[],
  options: ServerOptions,
): WebHandler {
  return async request => {
    const requestSegments = splitPath(new URL(request.url).pathname);
    const allowedMethods = new Set<string>();
    for (const route of routes) {
      const params = matchRoute(route, requestSegments);
      if (!params) {
        continue;
      }
      if (route.operation.method !== request.method.toUpperCase()) {
        allowedMethods.add(route.operation.method);
        continue;
      }
      return executeOperation(
        route.operation,
        route.service,
        { request, resolveParams: () => decodeRouteParams(params) },
        options,
      );
    }
    if (allowedMethods.size > 0) {
      return Response.json(
        { code: 405, msg: 'method not allowed' },
        {
          headers: { Allow: [...allowedMethods].sort().join(', ') },
          status: 405,
        },
      );
    }
    return Response.json(
      { code: 404, msg: 'route not found' },
      { status: 404 },
    );
  };
}

function matchRoute<TService>(
  route: Route<TService>,
  requestSegments: string[],
): RouteParams | undefined {
  const params: RouteParams = {};
  for (let index = 0; index < route.segments.length; index += 1) {
    const expected = route.segments[index] as string;
    const parameter = parseParameter(expected);
    if (parameter?.catchAll) {
      params[parameter.name] = requestSegments.slice(index);
      return params;
    }
    const actual = requestSegments[index];
    if (actual === undefined) {
      return undefined;
    }
    if (parameter) {
      params[parameter.name] = actual;
    } else if (expected !== actual) {
      return undefined;
    }
  }
  return route.segments.length === requestSegments.length ? params : undefined;
}

function splitPath(path: string): string[] {
  return path.split('/').filter(Boolean);
}

function parseParameter(
  segment: string,
): { catchAll: boolean; name: string } | undefined {
  if (!segment.startsWith('{') || !segment.endsWith('}')) {
    return undefined;
  }
  const name = segment.slice(1, -1);
  return name.startsWith('*')
    ? { catchAll: true, name: name.slice(1) }
    : { catchAll: false, name };
}
