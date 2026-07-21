import { createServer } from 'node:http';

import { createRouter, XidlServerError } from 'xidl-typescript-server';
import {
  type HttpStreamApi,
  HttpStreamApiOperations,
} from './{{MODULE_NAME}}.server.js';

class MyHttpStreamApi implements HttpStreamApi {
  async *alerts(request: { district: string }): AsyncIterable<string> {
    yield `${request.district}:1`;
    yield `${request.district}:2`;
  }

  async upload_asset(
    stream: AsyncIterable<{ asset_id: string; chunk: number[] }>,
  ): Promise<string> {
    let assetId = '';
    for await (const item of stream) {
      assetId = item.asset_id;
    }
    return `uploaded:${assetId}`;
  }
}

const service = new MyHttpStreamApi();
const handler = createRouter(Object.values(HttpStreamApiOperations), service, {
  authorize(request, requirements) {
    const authorization = request.headers.get('Authorization');
    if (
      requirements.some(requirement => requirement.kind === 'http_basic') &&
      !authorization?.startsWith('Basic ')
    ) {
      throw new XidlServerError('Unauthorized', 401);
    }
    if (
      requirements.some(requirement => requirement.kind === 'http_bearer') &&
      !authorization?.startsWith('Bearer ')
    ) {
      throw new XidlServerError('Unauthorized', 401);
    }
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

server.listen(port, '127.0.0.1');
