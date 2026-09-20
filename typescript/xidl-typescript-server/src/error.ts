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

export function errorResponse(error: unknown): Response {
  if (error instanceof XidlServerError) {
    return jsonError(error.code, error.code, error.msg, error.headers);
  }
  if (isZodErrorLike(error)) {
    return jsonError(400, 400, 'invalid request', undefined, error.issues);
  }
  return jsonError(500, 500, String(error));
}

function jsonError(
  status: number,
  code: number,
  message: string,
  extraHeaders?: Record<string, string>,
  detail?: unknown,
): Response {
  const headers = new Headers({ 'Content-Type': 'application/json' });
  for (const [name, value] of Object.entries(extraHeaders ?? {})) {
    headers.set(name, value);
  }
  return new Response(JSON.stringify({ code, detail, msg: message }), {
    headers,
    status,
  });
}
