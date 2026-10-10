import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createRouter } from 'xidl-typescript-server';
import { EnvelopeApiClient } from './{{MODULE_NAME}}.client.js';
import {
  NotFound,
  NotModified,
  PreconditionFailed,
} from './{{MODULE_NAME}}.errors.js';
import { EnvelopeApiOperations } from './{{MODULE_NAME}}.server.js';

const handler = createRouter(Object.values(EnvelopeApiOperations), {
  delete_file(_if_match: string) {
    throw new PreconditionFailed({
      code: 412,
      etag: 'current-tag',
      msg: 'stale revision',
    });
  },
  get_file(key: string) {
    if (key === 'missing')
      throw new NotFound({ code: 404, msg: 'no such file' });
    const metadata = {
      cached: false,
      etag: 'meta-v1',
      tags: ['001', '"quoted"'],
    };
    if (key === 'raw')
      return {
        kind: 'OctetStream',
        value: new TextEncoder().encode('envelope-bytes'),
        ...metadata,
      };
    if (key === 'text')
      return { kind: 'Text', value: 'envelope-text', ...metadata };
    return { kind: 'Json', value: { etag: 'meta-v1', id: key }, ...metadata };
  },
  get_fresh() {
    throw new NotModified({ etag: 'same-tag' });
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
    const client = new EnvelopeApiClient(`http://127.0.0.1:${address.port}`);
    const meta = await client.get_file('meta');
    assert(meta.kind === 'Json');
    assert.equal(meta.value.id, 'meta');
    assert.equal(meta.etag, 'meta-v1');
    assert.deepEqual(meta.tags, ['001', '"quoted"']);
    assert.equal(meta.cached, false);
    const raw = await client.get_file('raw');
    assert(raw.kind === 'OctetStream');
    assert.deepEqual(raw.value, new TextEncoder().encode('envelope-bytes'));
    const text = await client.get_file('text');
    assert(text.kind === 'Text');
    assert.equal(text.value, 'envelope-text');
    await assert.rejects(client.get_file('missing'), NotFound);
    server.close(() => server.listen(port, '127.0.0.1'));
  } catch (error) {
    console.error(error);
    process.exit(1);
  }
});
