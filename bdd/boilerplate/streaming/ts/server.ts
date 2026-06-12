import { createServer } from 'node:http';
import {
  createStreamingServiceHandler,
  type StreamingService,
} from './{{MODULE_NAME}}.server.js';

class MyStreaming implements StreamingService {
  async *ticks(req: { count: number }): AsyncIterable<number> {
    for (let i = 0; i < req.count; i++) {
      yield i;
    }
  }
}
const handler = createStreamingServiceHandler(new MyStreaming());

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
