import { executeOperation } from './execute.js';
import type {
  NextRouteHandler,
  OperationDescriptor,
  ServerOptions,
} from './types.js';

export function createNextRoute<TService, THandler extends keyof TService>(
  operation: OperationDescriptor<TService, THandler>,
  service: TService,
  options: ServerOptions = {},
): NextRouteHandler {
  return async (request, routeContext) => {
    const params = await routeContext.params;
    return executeOperation(operation, service, request, params, options);
  };
}
