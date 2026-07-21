import { XidlServerError } from './error.js';

export function normalizeMime(value: string): string {
  return value.split(';')[0]?.trim().toLowerCase() || 'application/json';
}

export function assertRequestContentType(
  headers: Headers,
  expectedContentType: string,
): void {
  const actual = headers.get('Content-Type');
  if (!actual || !expectedContentType) {
    return;
  }
  const expected = normalizeMime(expectedContentType);
  if (normalizeMime(actual) !== expected) {
    throw new XidlServerError(
      `unsupported request content type: ${normalizeMime(actual)}`,
      415,
    );
  }
}

export function assertAccepts(headers: Headers, contentType: string): void {
  const raw = headers.get('Accept');
  if (!raw || !contentType) {
    return;
  }
  const expected = normalizeMime(contentType);
  const accepted = raw.split(',').map(value => normalizeMime(value));
  const [expectedType] = expected.split('/');
  if (
    accepted.some(
      value =>
        value === '*/*' || value === expected || value === `${expectedType}/*`,
    )
  ) {
    return;
  }
  throw new XidlServerError(`not acceptable: expected ${expected}`, 406);
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

export function readRequestCookies(headers: Headers): Map<string, string[]> {
  const values = new Map<string, string[]>();
  const raw = headers.get('Cookie');
  if (!raw) {
    return values;
  }
  for (const part of raw.split(';')) {
    const item = part.trim();
    const separator = item.indexOf('=');
    if (separator < 0) {
      continue;
    }
    const name = item.slice(0, separator).trim();
    const value = decodeURIComponent(item.slice(separator + 1));
    values.set(name, [...(values.get(name) ?? []), value]);
  }
  return values;
}
