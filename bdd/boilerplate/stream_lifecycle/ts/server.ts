import { createServer } from 'node:http';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { createRouter } from 'xidl-typescript-server';
import { Conflict, NotModified, Retired } from './{{MODULE_NAME}}.errors.js';
import type { LedgerImportRowsRequest } from './{{MODULE_NAME}}.iface.js';
import { type Ledger, LedgerOperations } from './{{MODULE_NAME}}.server.js';

const service: Ledger = {
  async *export_rows(job: string) {
    if (job === 'locked')
      throw new Conflict({ detail: { job, reason: 'locked' } });
    if (job === 'retired') throw new Retired({});
    if (job === 'cached') throw new NotModified({});
    if (job === 'empty') return;
    yield { id: 'zero', quantity: 0 };
    if (job === 'partial')
      throw new Conflict({ detail: { job, reason: 'locked' } });
    if (job === 'partial-cached') throw new NotModified({});
    yield { id: '雪', quantity: 2 };
  },
  async import_rows(stream: AsyncIterable<LedgerImportRowsRequest>) {
    const ids = new Set<string>();
    let total = 0;
    for await (const { row } of stream) {
      if (row.quantity < 0 || ids.has(row.id)) {
        throw new Conflict({
          detail: { job: row.id, reason: 'duplicate or negative quantity' },
        });
      }
      ids.add(row.id);
      total += row.quantity;
    }
    return { accepted: ids.size, total };
  },
};
const handler = createRouter(Object.values(LedgerOperations), service);
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
