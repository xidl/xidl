import type { VersionParameterSpec } from './versioned.ts';

export type ExtractedParameter =
  | { kind: 'ok'; value: string | undefined }
  | {
      kind: 'error';
      error: 'repeated' | 'malformed';
      message: string;
      rawValue: string;
    };

export function extractVersionParameter(
  request: Request,
  parameter: VersionParameterSpec,
): ExtractedParameter {
  if (parameter.in === 'header') {
    const raw = request.headers.get(parameter.name);
    if (raw === null) {
      return { kind: 'ok', value: undefined };
    }
    if (raw.includes(',')) {
      return {
        error: 'repeated',
        kind: 'error',
        message: `repeated version header '${parameter.name}' is not allowed`,
        rawValue: raw,
      };
    }
    const trimmed = raw.trim();
    return trimmed.length === 0
      ? {
          error: 'malformed',
          kind: 'error',
          message: `malformed version header '${parameter.name}'`,
          rawValue: raw,
        }
      : { kind: 'ok', value: trimmed };
  }

  if (parameter.in === 'query') {
    const values = new URL(request.url).searchParams.getAll(parameter.name);
    if (values.length === 0) {
      return { kind: 'ok', value: undefined };
    }
    if (values.length > 1) {
      return {
        error: 'repeated',
        kind: 'error',
        message: `repeated version query parameter '${parameter.name}' is not allowed`,
        rawValue: values.join(', '),
      };
    }
    const rawValue = values[0] ?? '';
    const value = rawValue.trim();
    return value.length === 0
      ? {
          error: 'malformed',
          kind: 'error',
          message: `malformed version query parameter '${parameter.name}'`,
          rawValue,
        }
      : { kind: 'ok', value };
  }

  return extractVersionCookie(request, parameter.name);
}

function extractVersionCookie(
  request: Request,
  name: string,
): ExtractedParameter {
  const cookieHeader = request.headers.get('cookie');
  if (!cookieHeader) {
    return { kind: 'ok', value: undefined };
  }
  const matches: string[] = [];
  for (const part of cookieHeader.split(';')) {
    const eq = part.indexOf('=');
    if (eq === -1 || part.slice(0, eq).trim() !== name) {
      continue;
    }
    const rawValue = part.slice(eq + 1).trim();
    try {
      matches.push(decodeURIComponent(rawValue));
    } catch {
      return {
        error: 'malformed',
        kind: 'error',
        message: `malformed version cookie '${name}'`,
        rawValue,
      };
    }
  }
  if (matches.length === 0) {
    return { kind: 'ok', value: undefined };
  }
  if (matches.length > 1) {
    return {
      error: 'repeated',
      kind: 'error',
      message: `repeated version cookie '${name}' is not allowed`,
      rawValue: matches.join(', '),
    };
  }
  const rawValue = matches[0] ?? '';
  const value = rawValue.trim();
  return value.length === 0
    ? {
        error: 'malformed',
        kind: 'error',
        message: `malformed version cookie '${name}'`,
        rawValue,
      }
    : { kind: 'ok', value };
}

export function varyTokenForParameter(
  parameter: VersionParameterSpec,
): string | undefined {
  if (parameter.in === 'header') {
    return parameter.name;
  }
  if (parameter.in === 'cookie') {
    return 'Cookie';
  }
  return undefined;
}

export function withVary(response: Response, token?: string): Response {
  if (!token || response.status === 0) {
    return response;
  }
  const existing = response.headers.get('Vary');
  const tokens = new Map<string, string>();
  for (const item of (existing ?? '').split(',')) {
    const value = item.trim();
    const lower = value.toLowerCase();
    if (value && !tokens.has(lower)) {
      tokens.set(lower, value);
    }
  }
  const lower = token.toLowerCase();
  if (!tokens.has(lower)) {
    tokens.set(lower, token);
  }
  const vary = tokens.has('*') ? '*' : [...tokens.values()].join(', ');
  if (vary === existing) {
    return response;
  }
  const headers = new Headers(response.headers);
  headers.set('Vary', vary);
  return new Response(response.body, {
    headers,
    status: response.status,
    statusText: response.statusText,
  });
}
