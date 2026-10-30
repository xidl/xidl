/** The immutable terminal outcome of a structured HTTP stream. */
export type StreamOutcome =
  | 'Completed'
  | 'Failed'
  | 'Cancelled'
  | 'Interrupted';

/** The same status and JSON body used by the operation's HTTP error response. */
export interface StreamFailure {
  status: number;
  body: unknown;
}

/** Control frames are independent of the business item schema. */
export type StreamFrame =
  | { t: 'next'; data: unknown; seq?: number }
  | { t: 'error'; error: StreamFailure }
  | { t: 'complete' }
  | { t: 'heartbeat' };

export class StreamInterruptedError extends Error {
  constructor(message = 'stream ended without a terminal frame') {
    super(message);
    this.name = 'StreamInterruptedError';
  }
}

/** Finishing or cleaning up a stream never overwrites its terminal outcome. */
export class StreamLifecycle {
  private terminal?: StreamOutcome;

  get outcome(): StreamOutcome | undefined {
    return this.terminal;
  }

  finish(outcome: StreamOutcome): void {
    this.terminal ??= outcome;
  }

  interrupted(message?: string): never {
    this.finish('Interrupted');
    throw new StreamInterruptedError(message);
  }
}

export function decodeStreamFrame(raw: unknown): StreamFrame {
  if (!raw || typeof raw !== 'object') {
    throw new Error('stream frame must be an object');
  }
  const frame = raw as Record<string, unknown>;
  if (frame.t === 'complete' || frame.t === 'heartbeat') {
    return { t: frame.t };
  }
  if (frame.t === 'next' && Object.hasOwn(frame, 'data')) {
    return { data: frame.data, t: 'next' };
  }
  if (frame.t === 'error' && frame.error && typeof frame.error === 'object') {
    const error = frame.error as Record<string, unknown>;
    if (
      typeof error.status === 'number' &&
      Number.isInteger(error.status) &&
      error.status >= 300 &&
      error.status <= 599 &&
      Object.hasOwn(error, 'body')
    ) {
      return { error: { body: error.body, status: error.status }, t: 'error' };
    }
  }
  throw new Error('invalid stream control frame');
}

/** Pure SSE framing; comments and empty event blocks never become data. */
export class SseDecoder {
  private event?: string;
  private data: string[] = [];

  push(line: string): StreamFrame | undefined {
    if (line === '') {
      const event = this.event;
      const data = this.data;
      this.event = undefined;
      this.data = [];
      if (!data.length) return undefined;
      const payload = data.join('\n');
      if (event === 'complete') return { t: 'complete' };
      if (event === 'error') {
        return decodeStreamFrame({ error: JSON.parse(payload), t: 'error' });
      }
      if (!event || event === 'message' || event === 'next') {
        return { data: JSON.parse(payload), t: 'next' };
      }
      return undefined;
    }
    if (line.startsWith('event:')) this.event = line.slice(6).trim();
    if (line.startsWith('data:')) {
      const value = line.slice(5);
      this.data.push(value.startsWith(' ') ? value.slice(1) : value);
    }
    return undefined;
  }
}
