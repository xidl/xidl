import { ZodError } from 'zod';

export class XidlServerError extends Error {
  readonly code: number;
  readonly headers?: Record<string, string>;
  readonly status: number;

  constructor(
    message: string,
    status = 500,
    code = status,
    headers?: Record<string, string>,
  ) {
    super(message);
    this.status = status;
    this.code = code;
    this.headers = headers;
  }
}

export function errorResponse(error: unknown): Response {
  if (error instanceof XidlServerError) {
    return jsonError(error.status, error.code, error.message, error.headers);
  }
  if (error instanceof ZodError) {
    return jsonError(400, 400, 'invalid request', undefined, error.issues);
  }
  return jsonError(500, 500, 'internal server error');
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
