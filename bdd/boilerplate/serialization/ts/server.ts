import { createServer } from 'node:http';
import { createRouter } from 'xidl-typescript-server';
import {
  type SerializationTest,
  SerializationTestOperations,
} from './{{MODULE_NAME}}.server.js';

class MySerializationTest implements SerializationTest {
  async get_string(): Promise<string> {
    return 'hello';
  }
  async get_int(): Promise<number> {
    return 42;
  }
  async get_bool(): Promise<boolean> {
    return true;
  }
  async get_struct(): Promise<any> {
    return { name: 'world' };
  }
  async echo_string(req: { value: string }): Promise<string> {
    return req.value;
  }
  async echo_struct(req: { value: any }): Promise<any> {
    return req.value;
  }
}
const service = new MySerializationTest();
const handler = createRouter(
  Object.values(SerializationTestOperations),
  service,
);

const port = process.env.PORT ? parseInt(process.env.PORT, 10) : 8080;
const server = createServer(async (req, res) => {
  try {
    const protocol = (req.socket as any).encrypted ? 'https' : 'http';
    const fullUrl = `${protocol}://${req.headers.host}${req.url}`;
    const request = new Request(fullUrl, {
      body:
        req.method !== 'GET' && req.method !== 'HEAD'
          ? (req as any)
          : undefined,
      // @ts-expect-error
      duplex: 'half',
      headers: req.headers as any,
      method: req.method,
    });
    const response = await handler(request);
    console.log(`TS LOG: ${req.method} ${req.url} -> ${response.status}`);
    res.statusCode = response.status;
    for (const [key, value] of response.headers) {
      res.setHeader(key, value);
    }
    if (response.body) {
      for await (const chunk of response.body as any) {
        res.write(chunk);
      }
    }
    res.end();
  } catch (err) {
    console.error('TS LOG: Error', err);
    res.statusCode = 500;
    res.end(String(err));
  }
});
server.listen(port, '127.0.0.1', () => {
  console.log(`TS LOG: Server listening on ${port}`);
});
