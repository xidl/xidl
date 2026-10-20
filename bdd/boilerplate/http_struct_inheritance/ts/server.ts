import { createServer } from 'node:http';
import { createRouter } from 'xidl-typescript-server';
import { type Accounts, AccountsOperations } from './{{MODULE_NAME}}.server.js';

class MyAccounts implements Accounts {
  async participant(): Promise<{ id: string; role: string }> {
    return { id: 'u-1', role: 'admin' };
  }

  async echo(participant: { id: string; role: string }): Promise<{
    id: string;
    role: string;
  }> {
    return participant;
  }
}

const handler = createRouter(
  Object.values(AccountsOperations),
  new MyAccounts(),
);

const port = process.env.PORT ? Number.parseInt(process.env.PORT, 10) : 8080;
const server = createServer(async (request, response) => {
  try {
    const protocol = request.socket.encrypted ? 'https' : 'http';
    const url = new URL(
      request.url ?? '/',
      `${protocol}://${request.headers.host}`,
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
      for await (const chunk of webResponse.body as AsyncIterable<Uint8Array>) {
        response.write(chunk);
      }
    }
    response.end();
  } catch (error) {
    response.statusCode = 500;
    response.end(String(error));
  }
});

server.listen(port, '127.0.0.1');
