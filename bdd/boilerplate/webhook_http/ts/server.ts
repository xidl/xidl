import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createRouter } from 'xidl-typescript-server';
import { ReceiverClient } from './webhook_http.client.js';
import { Rejected } from './webhook_http.errors.js';
import { type Receiver, ReceiverOperations } from './webhook_http.server.js';

type Event = Parameters<ReceiverClient['receive']>[1];
const deliveries = new Map<string, Event>();
const service: Receiver = {
  accepted() {
    return deliveries.size;
  },
  receive(delivery_id, event) {
    if (!event.invoice_id || event.amount < 0)
      throw new Rejected({ reason: 'invalid invoice' });
    const previous = deliveries.get(delivery_id);
    if (previous && JSON.stringify(previous) !== JSON.stringify(event)) {
      throw new Rejected({ reason: 'delivery payload conflict' });
    }
    deliveries.set(delivery_id, event);
    return {
      accepted: true,
      event_type: event.event_type,
      invoice_id: event.invoice_id,
    };
  },
};
const handler = createRouter(Object.values(ReceiverOperations), service);
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
    const client = new ReceiverClient(`http://127.0.0.1:${address.port}`);
    const event = {
      amount: 1,
      event_type: 'arbitrary.sdk.value',
      invoice_id: 'sdk',
    };
    const expected = {
      accepted: true,
      event_type: event.event_type,
      invoice_id: event.invoice_id,
    };
    for (let replay = 0; replay < 2; replay++) {
      assert.deepEqual(await client.receive('sdk-delivery', event), expected);
    }
    assert.equal(await client.accepted(), 1);
    await assert.rejects(
      client.receive('sdk-delivery', { ...event, amount: 2 }),
      (error: unknown) =>
        error instanceof Rejected &&
        error.reason === 'delivery payload conflict',
    );
    deliveries.clear();
    server.close(() => server.listen(port, '127.0.0.1'));
  } catch (error) {
    console.error(error);
    process.exit(1);
  }
});
