export type { XidlSchema } from 'xidl-typescript-codec';
export {
  type StreamFailure,
  StreamInterruptedError,
  StreamLifecycle,
  type StreamOutcome,
} from 'xidl-typescript-codec';
export { XidlClientError } from './error.ts';
export { resolveFetch, resolveFetchLegacy } from './fetch.ts';
export {
  appendCookie,
  appendHeader,
  appendParam,
  applyClientAuth,
  applyCookies,
  encodeRequestBody,
} from './request.ts';
export {
  buildResponsePayload,
  decodeOptionalResponseBody,
  decodeResponseBody,
  decodeXidlError,
  parseXidlError,
  readResponseCookies,
  readResponseHeader,
} from './response.ts';
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
} from './scalar.ts';
export {
  byteResponseStream,
  byteStreamBody,
  ndjsonBody,
  sseJsonStream,
} from './stream.ts';
export type {
  ClientAuth,
  ClientOptions,
  FetchLike,
  HttpCodec,
  ResponseValueSpec,
  SecurityRequirement,
} from './types.ts';
export type {
  OpenWsOptions,
  WebSocketConnectRequest,
  WebSocketLike,
  WsBidiSession,
} from './websocket.ts';
export { openWsBidiClient } from './websocket.ts';
