import { XidlServerError } from './error.ts';

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
      415,
      `unsupported request content type: ${normalizeMime(actual)}`,
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
  throw new XidlServerError(406, `not acceptable: expected ${expected}`);
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
  let seenPair = false;
  for (const part of raw.split(';')) {
    const item = part.trim();
    if (item.length === 0) {
      continue;
    }
    const separator = item.indexOf('=');
    if (separator < 0) {
      // The Rust cookie crate treats a leading bare word as a missing pair
      // and later bare words as attributes.
      if (!seenPair) {
        throw new XidlServerError(400, `malformed cookie pair: ${item}`);
      }
      continue;
    }
    const name = item.slice(0, separator).trim();
    if (name.length === 0) {
      throw new XidlServerError(400, 'malformed cookie pair: empty name');
    }
    // Values keep surrounding quotes and trim around '=' exactly like the
    // cookie crate; invalid percent escapes stay literal.
    let value = item.slice(separator + 1).trim();
    try {
      value = decodeURIComponent(value);
    } catch {
      // percent-encoding keeps invalid escapes verbatim
    }
    seenPair = true;
    values.set(name, [...(values.get(name) ?? []), value]);
  }
  return values;
}
