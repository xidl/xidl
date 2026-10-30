import { createServer } from 'node:http';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { createRouter } from 'xidl-typescript-server';
import { Conflict } from './{{MODULE_NAME}}.errors.js';
import type { catalog } from './{{MODULE_NAME}}.js';
import { type Store, StoreOperations } from './{{MODULE_NAME}}.server.js';

const item = (): catalog.Item => ({
  availability: 'inStock',
  initial: 'A',
  letters: ['A', '雪', '🚀'],
  mark: '🚀',
  revision: 9007199254740991,
  sku: 'sku-雪',
  stock_grid: [
    [0, 1, 2],
    [3, 4, 5],
  ],
  thumbnail: [0, 127, 255],
});
const conflict = () =>
  new Conflict({ reason: 'inventory locked', sku: 'locked' });
const service: Store = {
  async apply(change) {
    return change;
  },
  async bindings(count, initial, session) {
    return {
      count_reply: count,
      receipt: session,
      return: `${count}:${initial}:${session}`,
    };
  },
  async echo(value) {
    if (value.sku === 'locked') throw conflict();
    return value;
  },
  async *feed(state) {
    if (state === 'locked') throw conflict();
    yield { data: item(), tag: '0' };
    if (state === 'partial') throw conflict();
    yield { data: 'retired', tag: 'default' };
  },
  async label(change) {
    return change;
  },
};
const handler = createRouter(Object.values(StoreOperations), service);
const server = createServer(async (request, response) => {
  const init: RequestInit & { duplex: 'half' } = {
    duplex: 'half',
    headers: new Headers(
      Object.entries(request.headers).flatMap(([key, value]) =>
        value === undefined
          ? []
          : (Array.isArray(value) ? value : [value]).map(
              item => [key, item] as [string, string],
            ),
      ),
    ),
    method: request.method,
  };
  if (request.method === 'POST')
    init.body = Readable.toWeb(request) as ReadableStream<Uint8Array>;
  const result = await handler(
    new Request(`http://${request.headers.host}${request.url}`, init),
  );
  response.writeHead(result.status, Object.fromEntries(result.headers));
  if (result.body) {
    const reader = result.body.getReader();
    await pipeline(
      Readable.from(
        (async function* () {
          try {
            while (true) {
              const item = await reader.read();
              if (item.done) return;
              yield item.value;
            }
          } finally {
            await reader.cancel();
            reader.releaseLock();
          }
        })(),
      ),
      response,
    );
  } else response.end();
});
server.listen(Number(process.env.PORT ?? 8080), '127.0.0.1');
