import { StreamInterruptedError } from 'xidl-typescript-codec';
import type { TypedErrorResponse } from './types.ts';

/** An application-side helper for producing an HTTP response at the host boundary. */
export class XidlServerError extends Error {
  readonly code: number;
  readonly msg: string;
  readonly headers?: Record<string, string>;

  constructor(code: number, msg: string, headers?: Record<string, string>) {
    super(msg);
    this.code = code;
    this.msg = msg;
    this.headers = headers;
  }
}

/** Identifies a rejection produced while interpreting an incoming request. */
export class RequestFailure extends Error {
  constructor(
    readonly code: number,
    message: string,
    readonly detail?: readonly unknown[],
  ) {
    super(message);
  }

  static from(error: unknown): RequestFailure | undefined {
    if (error instanceof RequestFailure) {
      return error;
    }
    if (error instanceof RequestStreamFailure) {
      return error.failure;
    }
    return undefined;
  }

  response(): Response {
    return Response.json(
      {
        code: this.code,
        msg: this.message,
        ...(this.detail === undefined ? {} : { detail: this.detail }),
      },
      { status: this.code },
    );
  }
}

/** An NDJSON request failed while the service was reading its input stream. */
export class RequestStreamFailure extends StreamInterruptedError {
  constructor(readonly failure: RequestFailure) {
    super(failure.message);
  }
}

/** The service result failed its declared response schema. */
export class ResponseValidationError extends Error {
  constructor(readonly issues: readonly unknown[]) {
    super('invalid response');
    this.name = 'ResponseValidationError';
  }
}

export function declaredErrorResponse(error: TypedErrorResponse): Response {
  return new Response(
    error.body === undefined ? null : JSON.stringify(error.body),
    { headers: error.headers, status: error.status },
  );
}
