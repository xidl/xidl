import { StreamInterruptedError } from 'xidl-typescript-codec';
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

function isZodErrorLike(error: unknown): error is { issues: unknown[] } {
  if (typeof error !== 'object' || error === null) {
    return false;
  }
  const candidate = error as { issues?: unknown; name?: unknown };
  if (!Array.isArray(candidate.issues)) {
    return false;
  }
  if (typeof candidate.name === 'string' && candidate.name.includes('Zod')) {
    return true;
  }
  // Fall back to the issues shape alone so validation errors from a
  // different Zod copy are still mapped to 400.
  return true;
}

export function projectError(error: unknown): {
  status: number;
  body: unknown;
} {
  if (error instanceof StreamInterruptedError) {
    return { body: { code: 400, msg: error.message }, status: 400 };
  }
  if (error instanceof XidlServerError) {
    return { body: { code: error.code, msg: error.msg }, status: error.code };
  }
  if (isZodErrorLike(error)) {
    return {
      body: { code: 400, detail: error.issues, msg: 'invalid request' },
      status: 400,
    };
  }
  return { body: { code: 500, msg: String(error) }, status: 500 };
}

export function errorResponse(error: unknown): Response {
  const failure = projectError(error);
  const headers = new Headers(
    error instanceof XidlServerError ? error.headers : undefined,
  );
  headers.set('Content-Type', 'application/json');
  return new Response(JSON.stringify(failure.body), {
    headers,
    status: failure.status,
  });
}
