import { deserialize, serialize, type XidlSchema } from 'xidl-typescript-codec';

/** One application message equals one WebSocket Text frame (JSON codec). */
export interface WsBidiSession<TIn, TOut> {
  send(value: TIn): Promise<void>;
  recv(): Promise<TOut | null>;
  close(code?: number, reason?: string): Promise<void>;
  readonly subprotocol: string | null;
}

export interface OpenWsOptions {
  subprotocol?: string;
  protocols?: string[];
  WebSocketImpl?: typeof WebSocket;
  inSchema?: XidlSchema;
  outSchema?: XidlSchema;
}

type WsLike = {
  send(data: string): void;
  close(code?: number, reason?: string): void;
  addEventListener(type: string, listener: (event: unknown) => void): void;
  readyState: number;
  protocol: string;
};

function resolveWebSocketCtor(options?: OpenWsOptions): typeof WebSocket {
  if (options?.WebSocketImpl) {
    return options.WebSocketImpl;
  }
  const g = globalThis as { WebSocket?: typeof WebSocket };
  if (!g.WebSocket) {
    throw new Error('WebSocket is not available; pass options.WebSocketImpl');
  }
  return g.WebSocket;
}

/** Open a typed full-duplex WebSocket session to `url`. */
export async function openWsBidiClient<TIn, TOut>(
  url: string,
  options?: OpenWsOptions,
): Promise<WsBidiSession<TIn, TOut>> {
  const Ctor = resolveWebSocketCtor(options);
  const protocols =
    options?.protocols ??
    (options?.subprotocol ? [options.subprotocol] : undefined);
  const socket = protocols?.length ? new Ctor(url, protocols) : new Ctor(url);
  const ws = socket as unknown as WsLike;
  await new Promise<void>((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error('websocket connect timeout')),
      15000,
    );
    ws.addEventListener('open', () => {
      clearTimeout(timer);
      resolve();
    });
    ws.addEventListener('error', (err: unknown) => {
      clearTimeout(timer);
      reject(
        err instanceof Error ? err : new Error('websocket connect failed'),
      );
    });
  });

  const queue: TOut[] = [];
  let notify: ((value: TOut | null) => void) | null = null;
  let closed = false;

  ws.addEventListener('message', event => {
    const data = (event as { data?: unknown }).data;
    const raw = typeof data === 'string' ? data : String(data);
    const parsed = JSON.parse(raw);
    const value = (
      options?.outSchema ? deserialize(parsed, options.outSchema) : parsed
    ) as TOut;
    if (notify) {
      const resolve = notify;
      notify = null;
      resolve(value);
    } else {
      queue.push(value);
    }
  });
  ws.addEventListener('close', () => {
    closed = true;
    if (notify) {
      const resolve = notify;
      notify = null;
      resolve(null);
    }
  });

  return {
    async close(code = 1000, reason = '') {
      ws.close(code, reason);
    },
    async recv() {
      if (queue.length > 0) {
        return queue.shift() as TOut;
      }
      if (closed) {
        return null;
      }
      return new Promise<TOut | null>(resolve => {
        notify = resolve;
      });
    },
    async send(value: TIn) {
      const payload = options?.inSchema
        ? serialize(value, options.inSchema)
        : value;
      ws.send(JSON.stringify(payload));
    },
    subprotocol: ws.protocol || null,
  };
}
