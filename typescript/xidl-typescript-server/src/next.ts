import { executeOperation } from './execute.ts';
import type {
  NextRouteHandler,
  OperationDescriptor,
  ServerOptions,
} from './types.ts';

export function createNextRoute<TService, THandler extends keyof TService>(
  operation: OperationDescriptor<TService, THandler>,
  service: TService,
  options: ServerOptions = {},
): NextRouteHandler {
  return async (request, routeContext) => {
    return executeOperation(
      operation,
      service,
      request,
      () => routeContext.params,
      options,
    );
  };
}
