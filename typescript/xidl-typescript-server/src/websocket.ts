import { deserialize, serialize, type XidlSchema } from 'xidl-typescript-codec';

/** Typed full-duplex session backed by a WebSocket-like connection. */
export interface WsBidiServerSession<TIn, TOut> {
  read(): Promise<TIn | null>;
  write(value: TOut): Promise<void>;
  close(code?: number, reason?: string): void;
}

export type WsLikeSocket = {
  send(data: string): void;
  close(code?: number, reason?: string): void;
  on(event: string, listener: (...args: unknown[]) => void): void;
  readyState?: number;
  protocol?: string;
};

/**
 * Wrap an accepted WebSocket connection (for example from the `ws` package)
 * as a typed XIDL bidi session. One application message equals one Text frame.
 */
export function wrapWsBidiServer<TIn, TOut>(
  socket: WsLikeSocket,
  options?: { inSchema?: XidlSchema; outSchema?: XidlSchema },
): WsBidiServerSession<TIn, TOut> {
  const queue: TIn[] = [];
  let notify: ((value: TIn | null) => void) | null = null;
  let closed = false;

  socket.on('message', (...args: unknown[]) => {
    const data = args[0];
    const raw = typeof data === 'string' ? data : String(data);
    const parsed = JSON.parse(raw);
    const value = (
      options?.inSchema ? deserialize(parsed, options.inSchema) : parsed
    ) as TIn;
    if (notify) {
      const resolve = notify;
      notify = null;
      resolve(value);
    } else {
      queue.push(value);
    }
  });
  socket.on('close', () => {
    closed = true;
    if (notify) {
      const resolve = notify;
      notify = null;
      resolve(null);
    }
  });
  socket.on('error', () => {
    closed = true;
    if (notify) {
      const resolve = notify;
      notify = null;
      resolve(null);
    }
  });

  return {
    close(code = 1000, reason = '') {
      closed = true;
      socket.close(code, reason);
    },
    async read() {
      if (queue.length > 0) {
        return queue.shift() as TIn;
      }
      if (closed) {
        return null;
      }
      return new Promise<TIn | null>(resolve => {
        notify = resolve;
      });
    },
    async write(value: TOut) {
      const payload = options?.outSchema
        ? serialize(value, options.outSchema)
        : value;
      socket.send(JSON.stringify(payload));
    },
  };
}

export interface UpgradeWebSocketOptions {
  subprotocol?: string;
}

/**
 * Validate an HTTP upgrade request for RFC 6455 and pick a subprotocol.
 * Returns false when the request is not a valid WebSocket upgrade,
 * null when no subprotocol is required, or the selected subprotocol.
 */
export function selectWsSubprotocol(
  request: Request,
  options?: UpgradeWebSocketOptions,
): string | null | false {
  const upgrade = request.headers.get('upgrade') ?? '';
  if (!upgrade.toLowerCase().includes('websocket')) {
    return false;
  }
  if (!options?.subprotocol) {
    return null;
  }
  const offered = (request.headers.get('sec-websocket-protocol') ?? '')
    .split(',')
    .map(value => value.trim())
    .filter(Boolean);
  if (
    offered.length > 0 &&
    !offered.some(value => value === options.subprotocol)
  ) {
    return false;
  }
  return options.subprotocol;
}
