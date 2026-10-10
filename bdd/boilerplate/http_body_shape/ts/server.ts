import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createRouter } from 'xidl-typescript-server';
import { BodyShapeClient } from './{{MODULE_NAME}}.client.js';
import { BodyShapeOperations } from './{{MODULE_NAME}}.server.js';

const handler = createRouter(Object.values(BodyShapeOperations), {
  document: () => ({ etag: 'document-v1', return: { title: 'Guide' } }),
  output: () => ({ document: { title: 'Guide' }, etag: 'output-v1' }),
  rename: (document: { title: string }) => ({
    etag: 'renamed-v1',
    return: document,
  }),
  scalar: () => ({
    cached: false,
    count: 7,
    etag: 'scalar-v1',
    return: 'Guide',
  }),
  upload: (media_type: string | undefined, payload: Uint8Array) => ({
    media_type: media_type ?? '',
    size: payload.length,
  }),
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
    const client = new BodyShapeClient(`http://127.0.0.1:${address.port}`);
    assert.deepEqual(await client.document(), {
      etag: 'document-v1',
      return: { title: 'Guide' },
    });
    assert.deepEqual(await client.scalar(), {
      cached: false,
      count: 7,
      etag: 'scalar-v1',
      return: 'Guide',
    });
    assert.deepEqual(await client.output(), {
      document: { title: 'Guide' },
      etag: 'output-v1',
    });
    assert.deepEqual(await client.rename({ title: 'Revised' }), {
      etag: 'renamed-v1',
      return: { title: 'Revised' },
    });
    const uploaded = await client.upload(
      'text/markdown',
      new TextEncoder().encode('# Guide'),
    );
    assert.equal(uploaded.media_type, 'text/markdown');
    assert.equal(uploaded.size, 7);
    server.close(() => server.listen(port, '127.0.0.1'));
  } catch (error) {
    console.error(error);
    process.exit(1);
  }
});
