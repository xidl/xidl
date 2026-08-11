export function normalizeMime(value: string): string {
  return value.split(';')[0]?.trim().toLowerCase() || 'application/json';
}

export function encodeScalar(value: unknown): string {
  if (
    typeof value === 'string' ||
    typeof value === 'number' ||
    typeof value === 'boolean'
  ) {
    return String(value);
  }
  return JSON.stringify(value);
}

export function parseScalar(value: string): unknown {
  if (value === 'true') {
    return true;
  }
  if (value === 'false') {
    return false;
  }
  if (/^-?\d+(\.\d+)?$/.test(value)) {
    return Number(value);
  }
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

export function joinUrl(baseUrl: string, path: string): string {
  if (baseUrl.endsWith('/') && path.startsWith('/')) {
    return `${baseUrl}${path.slice(1)}`;
  }
  if (!baseUrl.endsWith('/') && !path.startsWith('/')) {
    return `${baseUrl}/${path}`;
  }
  return `${baseUrl}${path}`;
}

export function encodePathSegment(value: unknown): string {
  return encodeURIComponent(String(value));
}

export function encodePathCatchAll(value: unknown): string {
  return String(value)
    .split('/')
    .map(segment => encodeURIComponent(segment))
    .join('/');
}

export function encodeQueryValue(value: unknown): string {
  if (value === null || value === undefined) {
    return '';
  }
  return encodeScalar(value);
}

export function encodeHeaderValue(value: unknown): string {
  if (value === null || value === undefined) {
    return '';
  }
  return encodeScalar(value);
}

export function encodeCookieValue(value: unknown): string {
  if (value === null || value === undefined) {
    return '';
  }
  return encodeScalar(value);
}
