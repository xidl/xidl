import type { z } from 'zod';

export type FetchLike = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;

export interface HttpCodec {
  encode?: (value: unknown, schema?: z.ZodTypeAny) => BodyInit | null;
  decode?: <T>(resp: Response, schema?: z.ZodTypeAny) => Promise<T>;
}

export type ClientAuth =
  | { kind: 'basic'; username: string; password: string }
  | { kind: 'bearer'; token: string }
  | { kind: 'api_key'; value: string; name?: string; location?: string };

export interface ClientOptions {
  fetch?: FetchLike;
  headers?: Record<string, string>;
  auth?: ClientAuth;
  codecs?: Record<string, HttpCodec>;
}

export interface SecurityRequirement {
  kind: string;
  location?: string;
  name?: string;
  realm?: string;
}

export interface ResponseValueSpec {
  name: string;
  key: string;
  optional: boolean;
  isMulti: boolean;
}
