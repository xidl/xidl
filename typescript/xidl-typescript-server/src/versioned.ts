import { errorResponse, XidlServerError } from './error.ts';
import type { Awaitable, WebHandler } from './types.ts';
import {
  addVaryToken,
  extractVersionParameter,
  varyTokenForParameter,
  withVersionHeaders,
} from './version_extract.ts';

export type VersionLifecycle = 'preview' | 'latest' | 'maintained' | 'retired';

export type VersionParameterLocation = 'header' | 'query' | 'cookie';

export interface VersionParameterSpec {
  in: VersionParameterLocation;
  name: string;
}

export interface VersionEntry<TVersionId extends string = string> {
  /** Machine identifier of the version (for example `v2026_06_01`). */
  id: TVersionId;
  /** Wire value exchanged in HTTP requests and echoed in responses. */
  value: string;
  /** Lifecycle status; `'retired'` responds with `410 Gone`. */
  status?: VersionLifecycle;
  /** Handler mounted for this version (required unless status is `'retired'`). */
  handler?: WebHandler;
}

export type VersionErrorKind =
  | 'missing'
  | 'repeated'
  | 'malformed'
  | 'unknown'
  | 'retired';

export interface VersionResolutionError {
  kind: VersionErrorKind;
  parameter: VersionParameterSpec;
  rawValue?: string;
  status: 400 | 410;
  message: string;
  supportedVersions: readonly string[];
  retiredVersion?: string;
}

export type VersionResolutionOutcome<TVersionId extends string = string> =
  | TVersionId
  | { version: TVersionId }
  | { error: VersionErrorKind; message?: string }
  | undefined;

export type VersionResolver<TVersionId extends string = string> = (
  raw: string | undefined,
  context: {
    request: Request;
    versions: readonly VersionEntry<TVersionId>[];
  },
) => VersionResolutionOutcome<TVersionId>;

export interface VersionedRouterOptions<TVersionId extends string = string> {
  /** Version carrier declared by the contract. */
  parameter: VersionParameterSpec;
  /** Served and retired versions for this contract. */
  versions: readonly VersionEntry<TVersionId>[];
  /** Optional fallback version ID used when the request omits the parameter. */
  defaultVersion?: TVersionId;
  /** Optional custom resolver; defaults to exact matching on wire `value`. */
  resolveVersion?: VersionResolver<TVersionId>;
  /** Echo resolved version header and add `Vary`; defaults to `true` for headers. */
  echoHeader?: boolean;
  /** Optional custom response builder for version resolution failures. */
  onVersionError?: (
    error: VersionResolutionError,
    request: Request,
  ) => Awaitable<Response>;
}

interface VersionRouterIndex<TVersionId extends string> {
  byId: Map<TVersionId, VersionEntry<TVersionId>>;
  byValue: Map<string, VersionEntry<TVersionId>>;
  options: VersionedRouterOptions<TVersionId>;
  supportedVersions: readonly string[];
}

/**
 * Mounts every version of an HTTP contract behind a single Web-standard handler.
 */
export function createVersionedRouter<TVersionId extends string = string>(
  options: VersionedRouterOptions<TVersionId>,
): WebHandler {
  const byId = new Map<TVersionId, VersionEntry<TVersionId>>();
  const byValue = new Map<string, VersionEntry<TVersionId>>();
  const supportedVersions: string[] = [];
  for (const entry of options.versions) {
    if (byId.has(entry.id)) {
      throw new Error(`duplicate version id '${entry.id}'`);
    }
    if (byValue.has(entry.value)) {
      throw new Error(`duplicate version value '${entry.value}'`);
    }
    byId.set(entry.id, entry);
    byValue.set(entry.value, entry);
    if (entry.status !== 'retired' && entry.handler) {
      supportedVersions.push(entry.value);
    }
  }
  if (
    options.defaultVersion !== undefined &&
    !byId.has(options.defaultVersion)
  ) {
    throw new Error(
      `defaultVersion '${options.defaultVersion}' is not declared in versions`,
    );
  }
  const index: VersionRouterIndex<TVersionId> = {
    byId,
    byValue,
    options,
    supportedVersions,
  };
  const echoHeader = options.echoHeader ?? options.parameter.in === 'header';
  const varyToken = echoHeader
    ? varyTokenForParameter(options.parameter)
    : undefined;

  return async request => {
    try {
      const extracted = extractVersionParameter(request, options.parameter);
      if (extracted.kind === 'error') {
        return await respondVersionError(
          {
            kind: extracted.error,
            message: extracted.message,
            parameter: options.parameter,
            rawValue: extracted.rawValue,
            status: 400,
            supportedVersions,
          },
          request,
          options,
          varyToken,
        );
      }

      const resolved = resolveEntry(extracted.value, request, index);
      if ('error' in resolved) {
        return await respondVersionError(
          resolved.error,
          request,
          options,
          varyToken,
        );
      }

      const { entry } = resolved;
      if (entry.status === 'retired') {
        return await respondVersionError(
          {
            kind: 'retired',
            message: `version '${entry.value}' is retired`,
            parameter: options.parameter,
            rawValue: extracted.value,
            retiredVersion: entry.value,
            status: 410,
            supportedVersions,
          },
          request,
          options,
          varyToken,
        );
      }

      if (!entry.handler) {
        return errorResponse(
          new XidlServerError(
            500,
            `version '${entry.value}' (${entry.id}) has no handler configured`,
          ),
        );
      }

      const response = await entry.handler(request);
      if (!echoHeader) {
        return response;
      }
      return withVersionHeaders(
        response,
        options.parameter.name,
        entry.value,
        varyToken,
      );
    } catch (error) {
      return errorResponse(error);
    }
  };
}

function resolveEntry<TVersionId extends string>(
  raw: string | undefined,
  request: Request,
  index: VersionRouterIndex<TVersionId>,
): { entry: VersionEntry<TVersionId> } | { error: VersionResolutionError } {
  const { byId, byValue, options, supportedVersions } = index;
  if (options.resolveVersion) {
    const outcome = options.resolveVersion(raw, {
      request,
      versions: options.versions,
    });
    if (outcome === undefined) {
      const kind: VersionErrorKind = raw === undefined ? 'missing' : 'unknown';
      return {
        error: {
          kind,
          message:
            raw === undefined
              ? `missing required version ${options.parameter.in} '${options.parameter.name}'`
              : `unsupported version '${raw}'`,
          parameter: options.parameter,
          rawValue: raw,
          status: 400,
          supportedVersions,
        },
      };
    }
    if (typeof outcome === 'object' && 'error' in outcome) {
      const status = outcome.error === 'retired' ? 410 : 400;
      return {
        error: {
          kind: outcome.error,
          message:
            outcome.message ??
            (outcome.error === 'retired'
              ? `version '${raw ?? ''}' is retired`
              : `invalid version '${raw ?? ''}'`),
          parameter: options.parameter,
          rawValue: raw,
          retiredVersion: outcome.error === 'retired' ? raw : undefined,
          status,
          supportedVersions,
        },
      };
    }
    const id = typeof outcome === 'string' ? outcome : outcome.version;
    const entry = byId.get(id);
    if (!entry) {
      return {
        error: {
          kind: 'unknown',
          message: `unsupported version '${raw ?? id}'`,
          parameter: options.parameter,
          rawValue: raw,
          status: 400,
          supportedVersions,
        },
      };
    }
    return { entry };
  }

  if (raw === undefined) {
    const fallback =
      options.defaultVersion !== undefined
        ? byId.get(options.defaultVersion)
        : undefined;
    if (fallback) {
      return { entry: fallback };
    }
    return {
      error: {
        kind: 'missing',
        message: `missing required version ${options.parameter.in} '${options.parameter.name}'`,
        parameter: options.parameter,
        status: 400,
        supportedVersions,
      },
    };
  }

  const entry = byValue.get(raw);
  if (!entry) {
    return {
      error: {
        kind: 'unknown',
        message: `unsupported version '${raw}'`,
        parameter: options.parameter,
        rawValue: raw,
        status: 400,
        supportedVersions,
      },
    };
  }
  return { entry };
}

async function respondVersionError<TVersionId extends string>(
  error: VersionResolutionError,
  request: Request,
  options: VersionedRouterOptions<TVersionId>,
  varyToken: string | undefined,
): Promise<Response> {
  const response = options.onVersionError
    ? await options.onVersionError(error, request)
    : new Response(
        JSON.stringify({
          code: error.status,
          kind: error.kind,
          msg: error.message,
          supportedVersions: error.supportedVersions,
          ...(error.retiredVersion !== undefined
            ? { retiredVersion: error.retiredVersion }
            : {}),
        }),
        {
          headers: { 'Content-Type': 'application/json' },
          status: error.status,
        },
      );
  if (!varyToken) {
    return response;
  }
  const headers = new Headers(response.headers);
  addVaryToken(headers, varyToken);
  return new Response(response.body, {
    headers,
    status: response.status,
    statusText: response.statusText,
  });
}
