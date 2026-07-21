import { errorResponse } from './error.js';
import { decodeOperationRequest } from './request.js';
import { encodeOperationResponse } from './response.js';
import { assertAccepts } from './scalar.js';
import type {
  Awaitable,
  OperationDescriptor,
  RouteParams,
  ServerContext,
  ServerOptions,
} from './types.js';

type RuntimeMethod = (
  inputOrContext?: unknown,
  context?: ServerContext,
) => Awaitable<unknown>;

export async function executeOperation<
  TService,
  THandler extends keyof TService,
>(
  operation: OperationDescriptor<TService, THandler>,
  service: TService,
  request: Request,
  params: RouteParams,
  options: ServerOptions,
): Promise<Response> {
  const context: ServerContext = {
    operation: {
      handler: operation.handler,
      method: operation.method,
      path: operation.path,
    },
    params,
    request,
  };
  try {
    await options.authorize?.(request, operation.security, context);
    assertAccepts(request.headers, operation.response.contentType);
    const input = await decodeOperationRequest(
      operation,
      request,
      params,
      options.codecs ?? {},
    );
    const method = service[operation.handler];
    if (typeof method !== 'function') {
      throw new TypeError(
        `server handler '${String(operation.handler)}' is not callable`,
      );
    }
    const invoke = method as RuntimeMethod;
    const result =
      operation.request.kind === 'none'
        ? await invoke.call(service, context)
        : await invoke.call(service, input, context);
    return encodeOperationResponse(operation, result, options.codecs ?? {});
  } catch (error) {
    if (options.onError) {
      return options.onError(error, context);
    }
    return errorResponse(error);
  }
}
