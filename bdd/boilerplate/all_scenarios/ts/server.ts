import { createServer } from 'node:http';
import {
  type AllScenariosService,
  createAllScenariosServiceHandler,
} from './{{MODULE_NAME}}.server.js';

class MyAllScenarios implements AllScenariosService {
  private status = 'ACTIVE';
  async get_item(req: {
    id: number;
    filter: string;
    trace_id: string;
  }): Promise<string> {
    return `Item ${req.id} with ${req.filter} and ${req.trace_id}`;
  }
  async create_item(req: { name: string; payload: any }): Promise<number> {
    return 42;
  }
  async update_item(req: { id: number; metadata: any[] }): Promise<void> {}
  async delete_item(req: { id: number }): Promise<void> {}
  async upload_form(req: { key: string; value: string }): Promise<void> {}
  async secure_data(req: { auth: any }): Promise<string> {
    return 'Secret';
  }
  async get_attribute_system_status(): Promise<string> {
    return this.status;
  }
  async set_attribute_system_status(req: {
    system_status: string;
  }): Promise<void> {
    this.status = req.system_status;
  }
  async get_attribute_version(): Promise<string> {
    return '1.0.0';
  }
}
const handler = createAllScenariosServiceHandler(new MyAllScenarios());

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
