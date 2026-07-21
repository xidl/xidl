import type { ZodType } from 'zod';

export type Awaitable<T> = T | Promise<T>;

export type RouteParams = Record<string, string | string[] | undefined>;

export interface HttpCodec {
  encode?: (value: unknown, schema?: ZodType) => BodyInit | null;
  decode?: <T>(response: Response, schema?: ZodType) => Promise<T>;
}

export interface SecurityRequirement {
  kind: string;
  location?: string;
  name?: string;
  realm?: string;
  scopes: string[];
}

export interface ValueBinding {
  key: string;
  multi: boolean;
  wireName: string;
}

export interface PathBinding extends ValueBinding {
  catchAll: boolean;
  parameter: string;
}

export interface BodyField {
  key: string;
  wireName: string;
}

export interface RequestBodySpec {
  contentType: string;
  fields: BodyField[];
  kind: 'none' | 'value' | 'stream';
  schema?: ZodType;
  singleKey?: string;
}

export interface OperationRequestSpec {
  body: RequestBodySpec;
  cookies: ValueBinding[];
  headers: ValueBinding[];
  kind: 'none' | 'object' | 'stream';
  path: PathBinding[];
  query: ValueBinding[];
  schema?: ZodType;
}

export interface OperationResponseSpec {
  bodyFields: BodyField[];
  bodyMode: 'none' | 'object' | 'return';
  contentType: string;
  cookies: ValueBinding[];
  headers: ValueBinding[];
  schema?: ZodType;
  stream: boolean;
  streamSchema?: ZodType;
}

export interface OperationDescriptor<
  TService,
  THandler extends keyof TService = keyof TService,
> {
  handler: THandler;
  method: string;
  path: string;
  paths?: string[];
  request: OperationRequestSpec;
  response: OperationResponseSpec;
  security: SecurityRequirement[];
}

export function defineOperation<TService, THandler extends keyof TService>(
  operation: OperationDescriptor<TService, THandler>,
): OperationDescriptor<TService, THandler> {
  return operation;
}

export interface ServerContext {
  operation: {
    handler: PropertyKey;
    method: string;
    path: string;
  };
  params: Readonly<RouteParams>;
  request: Request;
}

export interface ServerOptions {
  authorize?: (
    request: Request,
    requirements: SecurityRequirement[],
    context: ServerContext,
  ) => Awaitable<void>;
  codecs?: Record<string, HttpCodec>;
  onError?: (error: unknown, context: ServerContext) => Awaitable<Response>;
}

export interface NextRouteContext {
  params: Promise<RouteParams>;
}

export type NextRouteHandler = (
  request: Request,
  context: NextRouteContext,
) => Promise<Response>;

export type WebHandler = (request: Request) => Promise<Response>;
