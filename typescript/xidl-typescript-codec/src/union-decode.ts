import type { XidlSchema } from './index.js';

type ValidationError = { issues: readonly unknown[] };
type UnionOption = XidlSchema & {
  safeParse(
    value: unknown,
  ): { success: true } | { success: false; error: ValidationError };
};

export type WireDecode = (value: unknown, schema: XidlSchema) => unknown;
export type UnionDecoder = (
  value: unknown,
  options: readonly UnionOption[],
  decode: WireDecode,
) => unknown;

/** A successful wire decode or a rejection returned by a union schema. */
export type WireDecodeResult =
  | { ok: true; value: unknown }
  | { ok: false; issues: readonly unknown[] };

class UnionRejection {
  constructor(readonly error: ValidationError) {}
}

function selectUnion(
  value: unknown,
  options: readonly UnionOption[],
  decode: WireDecode,
  continueAfter: (error: unknown) => unknown,
): unknown {
  let failure: unknown;
  for (const option of options) {
    try {
      const candidate = decode(value, option);
      const parsed = option.safeParse(candidate);
      if (parsed.success) return candidate;
      failure = new UnionRejection(parsed.error);
    } catch (error) {
      failure = continueAfter(error);
    }
  }
  throw failure;
}

export const deserializeUnion: UnionDecoder = (value, options, decode) => {
  try {
    return selectUnion(value, options, decode, error => error);
  } catch (error) {
    throw error instanceof UnionRejection ? error.error : error;
  }
};

export const decodeRequestUnion: UnionDecoder = (value, options, decode) =>
  selectUnion(value, options, decode, error => {
    if (error instanceof UnionRejection) return error;
    throw error;
  });

export function captureWireDecode(decode: () => unknown): WireDecodeResult {
  try {
    return { ok: true, value: decode() };
  } catch (error) {
    if (error instanceof UnionRejection) {
      return { issues: error.error.issues, ok: false };
    }
    throw error;
  }
}
