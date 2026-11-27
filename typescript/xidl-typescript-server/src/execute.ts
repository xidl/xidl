import { declaredErrorResponse, RequestFailure } from './error.ts';
import { decodeOperationRequest } from './request.ts';
import { encodeOperationResponse } from './response.ts';
import { assertAccepts } from './scalar.ts';
import type {
  Awaitable,
  OperationDescriptor,
  RouteParams,
  ServerContext,
  ServerOptions,
  TypedErrorResponse,
} from './types.ts';

type RuntimeMethod = (...args: unknown[]) => Awaitable<unknown>;
type InvocationResult =
  | { kind: 'returned'; value: unknown }
  | { kind: 'declared-error'; error: TypedErrorResponse };

interface OperationInvocation {
  request: Request;
  resolveParams: () => Awaitable<RouteParams>;
}

export async function executeOperation<
  TService,
  THandler extends keyof TService,
>(
  operation: OperationDescriptor<TService, THandler>,
  service: TService,
  invocation: OperationInvocation,
  options: ServerOptions,
): Promise<Response> {
  const { request, resolveParams } = invocation;
  const context: ServerContext = {
    operation: {
      handler: operation.handler,
      method: operation.method,
      path: operation.path,
    },
    params: {},
    request,
  };
  const codecs = options.codecs ?? {};
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
      codecs,
    );
    const invoked = await invokeService(operation, service, input);
    response =
      invoked.kind === 'declared-error'
        ? declaredErrorResponse(invoked.error)
        : await encodeOperationResponse(
            operation,
            invoked.value,
            request.headers,
            codecs,
          );
  } catch (error) {
    const failure = RequestFailure.from(error);
    if (!failure) {
      throw error;
    }
    response = failure.response();
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

async function invokeService<TService>(
  operation: OperationDescriptor<TService>,
  service: TService,
  input: unknown,
): Promise<InvocationResult> {
  const method = service[operation.handler];
  if (typeof method !== 'function') {
    throw new TypeError(
      `server handler '${String(operation.handler)}' is not callable`,
    );
  }
  const invoke = method as RuntimeMethod;
  const record = (input ?? {}) as Record<string, unknown>;
  const args =
    operation.request.kind === 'stream'
      ? [input]
      : operation.request.kind === 'object'
        ? (operation.request.args ?? []).map(key => record[key])
        : [];
  try {
    return { kind: 'returned', value: await invoke.call(service, ...args) };
  } catch (error) {
    if (!RequestFailure.from(error)) {
      const declared = operation.errors?.match(error);
      if (declared) {
        return { error: declared, kind: 'declared-error' };
      }
    }
    throw error;
  }
}
