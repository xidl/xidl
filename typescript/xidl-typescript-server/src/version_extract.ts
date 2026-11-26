import type { VersionParameterSpec } from './versioned.ts';

export type ExtractedParameter =
  | { kind: 'ok'; value: string | undefined }
  | {
      kind: 'error';
      error: 'repeated' | 'malformed';
      message: string;
      rawValue?: string;
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
    const value = (values[0] ?? '').trim();
    return value.length === 0
      ? {
          error: 'malformed',
          kind: 'error',
          message: `malformed version query parameter '${parameter.name}'`,
          rawValue: values[0],
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
  const value = (matches[0] ?? '').trim();
  return value.length === 0
    ? {
        error: 'malformed',
        kind: 'error',
        message: `malformed version cookie '${name}'`,
        rawValue: matches[0],
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

export function withVersionHeaders(
  response: Response,
  headerName: string,
  versionValue: string,
  varyToken?: string,
): Response {
  const headers = new Headers(response.headers);
  headers.set(headerName, versionValue);
  if (varyToken) {
    addVaryToken(headers, varyToken);
  }
  return new Response(response.body, {
    headers,
    status: response.status,
    statusText: response.statusText,
  });
}

export function addVaryToken(headers: Headers, token: string): void {
  const existing = headers.get('Vary');
  if (!existing) {
    headers.set('Vary', token);
    return;
  }
  const tokens = existing
    .split(',')
    .map(item => item.trim())
    .filter(Boolean);
  if (tokens.some(item => item.toLowerCase() === token.toLowerCase())) {
    return;
  }
  tokens.push(token);
  headers.set('Vary', tokens.join(', '));
}
