import { errorResponse } from './error.ts';
import { decodeOperationRequest } from './request.ts';
import { encodeOperationResponse } from './response.ts';
import { assertAccepts } from './scalar.ts';
import type {
  Awaitable,
  OperationDescriptor,
  RouteParams,
  ServerContext,
  ServerOptions,
} from './types.ts';

type RuntimeMethod = (input?: unknown) => Awaitable<unknown>;

export async function executeOperation<
  TService,
  THandler extends keyof TService,
>(
  operation: OperationDescriptor<TService, THandler>,
  service: TService,
  request: Request,
  resolveParams: () => Awaitable<RouteParams>,
  options: ServerOptions,
): Promise<Response> {
  const context: ServerContext = {
    operation: {
      handler: operation.handler,
      method: operation.method,
      path: operation.path,
    },
    params: {},
    request,
  };
  let response: Response;
  try {
    context.params = await resolveParams();
    await options.authorize?.(request, operation.security, context);
    if (!operation.response.representations?.length) {
      assertAccepts(request.headers, operation.response.contentType);
    }
    const input = await decodeOperationRequest(
      operation,
      request,
      context.params,
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
    response = encodeOperationResponse(
      operation,
      result,
      request.headers,
      options.codecs ?? {},
    );
  } catch (error) {
    try {
      const typed = operation.errors?.match(error);
      if (typed) {
        const headers = typed.headers ?? new Headers();
        response = new Response(
          typed.body === undefined ? null : JSON.stringify(typed.body),
          {
            headers,
            status: typed.status,
          },
        );
      } else if (options.onError) {
        response = await options.onError(error, context);
      } else {
        response = errorResponse(error);
      }
    } catch (encodingError) {
      response = errorResponse(encodingError);
    }
  }
  if (!operation.response.fixedHeaders) {
    return response;
  }
  const headers = new Headers(response.headers);
  for (const [name, value] of Object.entries(operation.response.fixedHeaders)) {
    headers.set(name, value);
  }
  return new Response(response.body, {
    headers,
    status: response.status,
    statusText: response.statusText,
  });
}
