export class XidlClientError extends Error {
  readonly code: number;
  readonly status: number;

  constructor(message: string, code: number, status: number) {
    super(message);
    this.code = code;
    this.status = status;
  }
}
