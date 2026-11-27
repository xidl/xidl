import { errorResponse } from './error.ts';
import type { Awaitable, WebHandler } from './types.ts';
import {
  extractVersionParameter,
  varyTokenForParameter,
  withVary,
} from './version_extract.ts';

export type VersionParameterLocation = 'header' | 'query' | 'cookie';

export interface VersionParameterSpec {
  in: VersionParameterLocation;
  name: string;
}

export interface VersionEntry<TVersionId extends string = string> {
  /** Machine identifier of the version (for example `v2026_06_01`). */
  id: TVersionId;
  /** Wire value exchanged in HTTP requests. */
  value: string;
  /** Handler mounted for this version. */
  handler: WebHandler;
}

interface VersionErrorContext {
  parameter: VersionParameterSpec;
  message: string;
  supportedVersions: readonly string[];
}

export type VersionResolutionError = VersionErrorContext &
  (
    | { kind: 'retired'; status: 410; rawValue: string }
    | { kind: 'missing'; status: 400; rawValue?: never }
    | {
        kind: 'unknown' | 'repeated' | 'malformed';
        status: 400;
        rawValue: string;
      }
  );

export type VersionErrorKind = VersionResolutionError['kind'];

export interface VersionedRouterOptions<TVersionId extends string = string> {
  /** Version carrier declared by the contract. */
  parameter: VersionParameterSpec;
  /** Versions mounted for this contract. */
  versions: readonly VersionEntry<TVersionId>[];
  /** Retired wire values, which respond with 410 Gone. */
  retired?: readonly string[];
  /** Mounted version ID used when the request omits the parameter. */
  defaultVersion?: NoInfer<TVersionId>;
  /** Optional replacement response for version resolution failures. */
  onVersionError?: (
    error: VersionResolutionError,
    request: Request,
  ) => Awaitable<Response>;
}

/**
 * Mounts every version of an HTTP contract behind a single Web-standard handler.
 */
export function createVersionedRouter<TVersionId extends string = string>(
  options: VersionedRouterOptions<TVersionId>,
): WebHandler {
  const { parameter, onVersionError } = options;
  const byId = new Map<TVersionId, VersionEntry<TVersionId>>();
  const byValue = new Map<string, VersionEntry<TVersionId>>();
  for (const entry of options.versions) {
    if (byId.has(entry.id)) {
      throw new Error(`duplicate version id '${entry.id}'`);
    }
    if (byValue.has(entry.value)) {
      throw new Error(`duplicate version value '${entry.value}'`);
    }
    byId.set(entry.id, entry);
    byValue.set(entry.value, entry);
  }
  const retired = new Set<string>();
  for (const value of options.retired ?? []) {
    if (byValue.has(value) || retired.has(value)) {
      throw new Error(`duplicate version value '${value}'`);
    }
    retired.add(value);
  }
  const fallback =
    options.defaultVersion === undefined
      ? undefined
      : byId.get(options.defaultVersion);
  if (options.defaultVersion !== undefined && fallback === undefined) {
    throw new Error(
      `defaultVersion '${options.defaultVersion}' is not declared in versions`,
    );
  }
  const supportedVersions = [...byValue.keys()];
  const varyToken = varyTokenForParameter(parameter);

  const route: WebHandler = async request => {
    const extracted = extractVersionParameter(request, parameter);
    const context = { parameter, supportedVersions };
    if (extracted.kind === 'error') {
      return respondVersionError(
        {
          ...context,
          kind: extracted.error,
          message: extracted.message,
          rawValue: extracted.rawValue,
          status: 400,
        },
        request,
        onVersionError,
      );
    }
    const rawValue = extracted.value;
    if (rawValue !== undefined && retired.has(rawValue)) {
      return respondVersionError(
        {
          ...context,
          kind: 'retired',
          message: `version '${rawValue}' is retired`,
          rawValue,
          status: 410,
        },
        request,
        onVersionError,
      );
    }
    const entry = rawValue === undefined ? fallback : byValue.get(rawValue);
    if (entry) {
      return entry.handler(request);
    }
    return respondVersionError(
      rawValue === undefined
        ? {
            ...context,
            kind: 'missing',
            message: `missing required version ${parameter.in} '${parameter.name}'`,
            status: 400,
          }
        : {
            ...context,
            kind: 'unknown',
            message: `unsupported version '${rawValue}'`,
            rawValue,
            status: 400,
          },
      request,
      onVersionError,
    );
  };

  return async request => {
    try {
      return withVary(await route(request), varyToken);
    } catch (error) {
      return withVary(errorResponse(error), varyToken);
    }
  };
}

function respondVersionError(
  error: VersionResolutionError,
  request: Request,
  handleError: VersionedRouterOptions['onVersionError'],
): Awaitable<Response> {
  if (handleError) {
    return handleError(error, request);
  }
  return new Response(
    JSON.stringify({
      code: error.status,
      kind: error.kind,
      msg: error.message,
      supportedVersions: error.supportedVersions,
      ...(error.kind === 'retired' ? { retiredVersion: error.rawValue } : {}),
    }),
    {
      headers: { 'Content-Type': 'application/json' },
      status: error.status,
    },
  );
}
