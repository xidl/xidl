import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createServer, type IncomingMessage } from 'node:http';
import WebSocket, { WebSocketServer } from 'ws';
import { createRouter, wrapWsBidiServer } from 'xidl-typescript-server';
import { v1, v2 } from './version_parameter.client.js';
import {
  v1$business$Conflict,
  v2$business$Conflict,
} from './version_parameter.errors.js';
import { v1 as serverV1, v2 as serverV2 } from './version_parameter.server.js';

const location = process.env.VERSION_LOCATION!;
const name = process.env.VERSION_NAME!;
const value = process.env.VERSION_VALUE!;

async function within<T>(stage: string, operation: Promise<T>): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      operation,
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () => reject(new Error(`${stage} exceeded 5 seconds`)),
          5000,
        );
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}
const timedFetch: typeof fetch = (input, init) =>
  fetch(input, { ...init, signal: AbortSignal.timeout(5000) });

type Observed = { url: URL; headers: Headers };
function headersOf(req: IncomingMessage): Headers {
  const headers = new Headers();
  for (const [name, value] of Object.entries(req.headers)) {
    for (const part of Array.isArray(value)
      ? value
      : value === undefined
        ? []
        : [value]) {
      headers.append(name, part);
    }
  }
  return headers;
}
function observe(req: IncomingMessage): Observed {
  return {
    headers: headersOf(req),
    url: new URL(req.url!, `http://${req.headers.host}`),
  };
}
function version(observed: Observed, expected: string | undefined) {
  let values: string[];
  if (location === 'header')
    values = observed.headers.has(name) ? [observed.headers.get(name)!] : [];
  else if (location === 'query')
    values = observed.url.searchParams.getAll(name);
  else
    values = (observed.headers.get('Cookie') ?? '')
      .split(';')
      .map(v => v.trim().split(/=(.*)/s))
      .filter(([key]) => key === name)
      .map(([, value]) => decodeURIComponent(value));
  assert.deepEqual(values, expected === undefined ? [] : [expected]);
}
function responseVersion(headers: Headers, expected: string) {
  assert.equal(headers.get(name), location === 'header' ? expected : null);
  assert.equal(headers.get('Set-Cookie'), null);
}

for (const [clientModule, serverModule, Conflict, expected] of [
  [v1.business, serverV1.business, v1$business$Conflict, '2026-04'],
  [v2.business, serverV2.business, v2$business$Conflict, value],
] as const) {
  const requests: Observed[] = [];
  const handshakes: Observed[] = [];
  const handler = createRouter(Object.values(serverModule.StoreOperations), {
    _get(id: number, search: string, theme: string) {
      assert.equal(search, 'a +/%');
      assert.equal(theme, 'dark');
      return { id: String(id) };
    },
    chat(_room: string, text: string) {
      return { reply: text };
    },
    fail(id: number) {
      if (id === 1) throw new Conflict({ message: 'conflict' });
      throw new Error('storage offline');
    },
    remove() {},
  });
  const health = createRouter(Object.values(serverModule.HealthOperations), {
    health: () => 'healthy',
  });
  const server = createServer(async (req, res) => {
    const observed = observe(req);
    requests.push(observed);
    const request = new Request(observed.url, {
      headers: observed.headers,
      method: req.method,
    });
    const response = await (observed.url.pathname === '/health'
      ? health(request)
      : handler(request));
    responseVersion(response.headers, expected);
    res.writeHead(response.status, Object.fromEntries(response.headers));
    res.end(Buffer.from(await response.arrayBuffer()));
  });
  const wss = new WebSocketServer({ noServer: true });
  const onSocket = serverModule.create_chatHandler(
    {
      async chat(session) {
        for (;;) {
          const message = await session.read();
          if (message === null) return;
          await session.write({ reply: `${message.text}:ack` });
        }
      },
    },
    { wrapWsBidiServer },
  );
  const shapeHandlers = new Map([
    ['/chat', onSocket],
    [
      '/scalar',
      serverModule.create_scalarHandler(
        {
          async scalar(session) {
            const message = await within('scalar server input', session.read());
            assert.ok(message);
            await session.write(message.text + ':scalar');
          },
        },
        { wrapWsBidiServer },
      ),
    ],
    [
      '/model',
      serverModule.create_modelHandler(
        {
          async model(session) {
            const message = await within('model server input', session.read());
            assert.ok(message);
            await session.write({ id: message.id });
          },
        },
        { wrapWsBidiServer },
      ),
    ],
    [
      '/sink',
      serverModule.create_sinkHandler(
        {
          async sink(session) {
            const message = await within('sink server input', session.read());
            assert.deepEqual(message, { text: 'stored' });
          },
        },
        { wrapWsBidiServer },
      ),
    ],
    [
      '/source',
      serverModule.create_sourceHandler(
        {
          async source(session) {
            await session.write('announcement');
          },
        },
        { wrapWsBidiServer },
      ),
    ],
  ]);
  wss.on('headers', headers => {
    for (const [key, value] of Object.entries(
      serverModule.StoreOperations.chat.response.fixedHeaders ?? {},
    )) {
      headers.push(`${key}: ${value}`);
    }
  });
  server.on('upgrade', (req, socket, head) => {
    handshakes.push(observe(req));
    const accept = shapeHandlers.get(observe(req).url.pathname);
    assert.ok(accept);
    wss.handleUpgrade(req, socket, head, accepted => {
      void accept(accepted);
    });
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  const address = server.address();
  assert.ok(address && typeof address !== 'string');
  const base = `http://127.0.0.1:${address.port}`;
  try {
    const cookie = `session=kept; ${name}=old; ${name}=older; Theme=case-kept`;
    const client = new clientModule.StoreClient(base, {
      fetch: timedFetch,
      headers: { Cookie: cookie, 'X-Version': 'caller-old-version' },
    });
    assert.equal(clientModule.StoreClient.API_VERSION, expected);
    assert.equal(clientModule.HealthClient.API_VERSION, expected);
    for (let i = 0; i < 2; i++) {
      assert.equal((await client._get(7, 'a +/%', 'dark')).id, '7');
      const observed = requests.shift()!;
      version(observed, expected);
      assert.equal(observed.url.searchParams.get('search'), 'a +/%');
      assert.match(observed.headers.get('Cookie')!, /session=kept/);
      assert.match(observed.headers.get('Cookie')!, /theme=dark/);
      assert.match(observed.headers.get('Cookie')!, /Theme=case-kept/);
    }
    await client.remove(7);
    version(requests.shift()!, expected);
    await assert.rejects(client.fail(1), Conflict);
    version(requests.shift()!, expected);
    await assert.rejects(client.fail(2));
    version(requests.shift()!, expected);
    assert.equal(
      await new clientModule.HealthClient(base, { fetch: timedFetch }).health(),
      'healthy',
    );
    version(requests.shift()!, expected);

    const wsBase = new URL(base.replace('http:', 'ws:') + '/');
    wsBase.searchParams.set('room', 'team +/%');
    if (location === 'query') {
      wsBase.searchParams.append(name, 'old');
      wsBase.searchParams.append(name, 'older');
    }
    let upgraded: Headers | undefined;
    const session = await within(
      'Node handshake',
      clientModule.open_chatSession(wsBase.toString(), {
        connect: ({ url, protocols, headers }) => {
          const socket = new WebSocket(url, protocols, { headers });
          socket.on('upgrade', response => {
            upgraded = headersOf(response);
          });
          return socket;
        },
        headers: { Cookie: cookie, 'X-Trace': 'kept', 'X-Version': 'old' },
      }),
    );
    version(handshakes.shift()!, expected);
    assert.ok(upgraded);
    responseVersion(upgraded, expected);
    await session.send({ text: 'hello' });
    assert.deepEqual(await within('Node message or close', session.recv()), {
      reply: 'hello:ack',
    });
    await session.close();
    assert.equal(await within('Node message or close', session.recv()), null);

    // The browser constructor really connects, but gets only URL/protocols.
    // Node's built-in WebSocket has the same browser API and cannot set headers.
    const browser = await within(
      'browser handshake',
      clientModule.open_chatSession(wsBase.toString()),
    );
    const observed = handshakes.shift()!;
    version(observed, location === 'query' ? expected : undefined);
    assert.equal(observed.url.searchParams.get('room'), 'team +/%');
    assert.equal(observed.headers.get('X-Trace'), null);
    await browser.send({ text: 'browser' });
    assert.deepEqual(await within('browser message or close', browser.recv()), {
      reply: 'browser:ack',
    });
    await browser.close();
    assert.equal(
      await within('browser message or close', browser.recv()),
      null,
    );

    const shapesBase = base.replace('http:', 'ws:');
    const scalar = await within(
      'scalar handshake',
      clientModule.open_scalarSession(shapesBase),
    );
    await scalar.send({ text: 'hello' });
    assert.equal(
      await within('scalar response', scalar.recv()),
      'hello:scalar',
    );
    assert.equal(await within('scalar server close', scalar.recv()), null);
    const model = await within(
      'model handshake',
      clientModule.open_modelSession(shapesBase),
    );
    await model.send({ id: 'object' });
    assert.deepEqual(await within('model response', model.recv()), {
      id: 'object',
    });
    assert.equal(await within('model server close', model.recv()), null);
    const sink = await within(
      'sink handshake',
      clientModule.open_sinkSession(shapesBase),
    );
    await sink.send({ text: 'stored' });
    assert.equal(await within('sink server close', sink.recv()), null);
    const source = await within(
      'source handshake',
      clientModule.open_sourceSession(shapesBase),
    );
    assert.equal(
      await within('source response', source.recv()),
      'announcement',
    );
    assert.equal(await within('source server close', source.recv()), null);
    for (let i = 0; i < 4; i++)
      version(handshakes.shift()!, location === 'query' ? expected : undefined);

    for (const requested of [
      undefined,
      expected === '2026-04' ? value : '2026-04',
      'unknown',
    ]) {
      const url = new URL(`${base}/health`);
      const headers = new Headers();
      if (requested !== undefined) {
        if (location === 'header') headers.set(name, requested);
        else if (location === 'query') url.searchParams.set(name, requested);
        else headers.set('Cookie', `${name}=${encodeURIComponent(requested)}`);
      }
      const response = await timedFetch(url, { headers });
      assert.equal(response.status, 200);
      responseVersion(response.headers, expected);
      await response.text();
      requests.shift();
    }
    for (const path of [
      '/items/not-a-number?search=x',
      '/items/%ZZ?search=x',
    ]) {
      const response = await timedFetch(base + path);
      assert.equal(response.status, 400);
      responseVersion(response.headers, expected);
      await response.text();
      requests.shift();
    }
    assert.equal(requests.length, 0);
    assert.equal(handshakes.length, 0);
  } finally {
    for (const socket of wss.clients) socket.terminate();
    wss.close();
    server.close();
    await within('HTTP server shutdown', once(server, 'close'));
  }
}
console.log(
  'TypeScript generated HTTP, Node WS and browser WS version workflows passed',
);
