import { errorResponse, XidlServerError } from './error.js';
import { executeOperation } from './execute.js';
import type {
  OperationDescriptor,
  RouteParams,
  ServerOptions,
  WebHandler,
} from './types.js';

interface Route<TService> {
  operation: OperationDescriptor<TService>;
  segments: string[];
}

export function createRouter<TService>(
  operations: readonly OperationDescriptor<TService>[],
  service: TService,
  options: ServerOptions = {},
): WebHandler {
  const routes = operations.flatMap(operation =>
    (operation.paths ?? [operation.path]).map(path => ({
      operation,
      segments: splitPath(path),
    })),
  );
  return async request => {
    try {
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
          service,
          request,
          params,
          options,
        );
      }
      if (allowedMethods.size > 0) {
        return errorResponse(
          new XidlServerError('method not allowed', 405, 405, {
            Allow: [...allowedMethods].sort().join(', '),
          }),
        );
      }
      return errorResponse(new XidlServerError('route not found', 404));
    } catch (error) {
      return errorResponse(error);
    }
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
      params[parameter.name] = requestSegments.slice(index).map(decodeSegment);
      return params;
    }
    const actual = requestSegments[index];
    if (actual === undefined) {
      return undefined;
    }
    if (parameter) {
      params[parameter.name] = decodeSegment(actual);
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

function decodeSegment(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    throw new XidlServerError(`invalid route segment: ${value}`, 400);
  }
}
