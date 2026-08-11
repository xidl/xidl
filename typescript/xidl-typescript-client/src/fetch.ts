import type { FetchLike } from './types.js';

export function resolveFetch(fetchImpl?: FetchLike): FetchLike {
  if (fetchImpl) {
    return fetchImpl;
  }
  if (typeof fetch === 'function') {
    return fetch.bind(globalThis);
  }
  throw new Error('fetch is not available');
}

export function resolveFetchLegacy(fetchImpl?: FetchLike): FetchLike {
  if (fetchImpl) {
    return fetchImpl;
  }
  const win = (globalThis as { window?: { fetch: FetchLike } }).window;
  if (!win?.fetch) {
    throw new Error('window.fetch is not available');
  }
  return win.fetch.bind(undefined);
}
