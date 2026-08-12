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

type RuntimeMethod = (input?: unknown) => Awaitable<unknown>;

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
    let result: unknown;
    if (operation.request.kind === 'none') {
      result = await invoke.call(service);
    } else if (operation.request.kind === 'stream') {
      // For client streams the decoded input is the raw request stream.
      result = await invoke.call(service, input);
    } else {
      const record = (input ?? {}) as Record<string, unknown>;
      const args = (operation.request.args ?? []).map(key => record[key]);
      result = await invoke.call(service, ...args);
    }
    return encodeOperationResponse(operation, result, options.codecs ?? {});
  } catch (error) {
    if (options.onError) {
      return options.onError(error, context);
    }
    return errorResponse(error);
  }
}
