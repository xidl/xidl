export { XidlClientError } from './error.js';
export { resolveFetch, resolveFetchLegacy } from './fetch.js';
export { ndjsonBodyLegacy, sseJsonStreamLegacy } from './legacy.js';
export {
  appendCookie,
  appendHeader,
  appendParam,
  applyClientAuth,
  applyCookies,
  encodeRequestBody,
} from './request.js';
export {
  buildResponsePayload,
  decodeOptionalResponseBody,
  decodeResponseBody,
  parseXidlError,
  readResponseCookies,
  readResponseHeader,
} from './response.js';
export {
  encodeCookieValue,
  encodeHeaderValue,
  encodePathCatchAll,
  encodePathSegment,
  encodeQueryValue,
  encodeScalar,
  joinUrl,
  normalizeMime,
  parseScalar,
} from './scalar.js';
export {
  byteResponseStream,
  byteStreamBody,
  ndjsonBody,
  sseJsonStream,
} from './stream.js';
export type {
  ClientAuth,
  ClientOptions,
  FetchLike,
  HttpCodec,
  ResponseValueSpec,
  SecurityRequirement,
} from './types.js';
