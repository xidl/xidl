import { createServer } from 'node:http';

import { createRouter } from 'xidl-typescript-server';
import {
  type ByteStreamService,
  ByteStreamServiceOperations,
} from './{{MODULE_NAME}}.server.js';

class MyByteStreamService implements ByteStreamService {
  async *download_bytes(): AsyncIterable<number[]> {
    yield Array.from(new TextEncoder().encode('hello '));
    yield Array.from(new TextEncoder().encode('world'));
  }

  async upload_bytes(stream: AsyncIterable<number[]>): Promise<string> {
    const bytes: number[] = [];
    for await (const chunk of stream) {
      bytes.push(...chunk);
    }
    return new TextDecoder().decode(Uint8Array.from(bytes));
  }
}

const handler = createRouter(
  Object.values(ByteStreamServiceOperations),
  new MyByteStreamService(),
);
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

server.listen(port, '127.0.0.1');
