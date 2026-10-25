import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createRouter } from 'xidl-typescript-server';
import { ProbesClient } from './wire_strict.client.js';
import { ProbesOperations } from './wire_strict.server.js';

const handler = createRouter(Object.values(ProbesOperations), {
  check(probe: { id: string; locked: boolean; minutes: number }) {
    return probe;
  },
  find(deep: boolean, count: number) {
    return { id: 'query', locked: deep, minutes: count };
  },
});
const port = process.env.PORT ? Number.parseInt(process.env.PORT, 10) : 8080;
const server = createServer(async (request, response) => {
  const url = new URL(
    request.url ?? '/',
    `http://${request.headers.host ?? 'localhost'}`,
  );
  const webRequest = new Request(url, {
    body:
      request.method !== 'GET' && request.method !== 'HEAD'
        ? (request as unknown as BodyInit)
        : undefined,
    // @ts-expect-error Node.js fetch requires duplex for streamed request bodies.
    duplex: 'half',
    headers: request.headers as HeadersInit,
    method: request.method,
  });
  const webResponse = await handler(webRequest);
  response.statusCode = webResponse.status;
  webResponse.headers.forEach((value, name) => {
    response.setHeader(name, value);
  });
  if (webResponse.body) {
    for await (const chunk of webResponse.body) {
      response.write(chunk);
    }
  }
  response.end();
});

server.listen(0, '127.0.0.1', async () => {
  try {
    const address = server.address();
    assert.ok(address && typeof address !== 'string');
    const base = `http://127.0.0.1:${address.port}`;
    const client = new ProbesClient(base);
    // The typed client round-trips exact values; the coercing query position
    // accepts the text the URL carries.
    assert.deepEqual(
      await client.check({ id: 'p1', locked: false, minutes: 7 }),
      {
        id: 'p1',
        locked: false,
        minutes: 7,
      },
    );
    assert.deepEqual(await client.find(true, 5), {
      id: 'query',
      locked: true,
      minutes: 5,
    });
    // A raw wrong-typed body is refused before the handler runs.
    const raw = await fetch(`${base}/probe`, {
      body: '{"id":"p1","locked":"false","minutes":1}',
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    });
    assert.equal(raw.status, 400, 'a string never becomes a boolean');
    await raw.text();
    server.close(() => server.listen(port, '127.0.0.1'));
  } catch (error) {
    console.error(error);
    process.exit(1);
  }
});
