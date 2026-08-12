import { createServer } from 'node:http';
import { createRouter } from 'xidl-typescript-server';
import type { Status } from './{{MODULE_NAME}}.js';
import {
  type AllScenariosService,
  AllScenariosServiceOperations,
} from './{{MODULE_NAME}}.server.js';

class MyAllScenarios implements AllScenariosService {
  private status: Status = 'ACTIVE';
  async get_item(
    id: number,
    filter: string,
    trace_id: string,
  ): Promise<string> {
    return `Item ${id} with ${filter} and ${trace_id}`;
  }
  async create_item(name: string, payload: any): Promise<number> {
    return 42;
  }
  async update_item(id: number, metadata: any[]): Promise<void> {}
  async delete_item(id: number): Promise<void> {}
  async upload_form(key: string, value: string): Promise<void> {}
  async secure_data(): Promise<string> {
    return 'Secret';
  }
  async get_attribute_system_status(): Promise<Status> {
    return this.status;
  }
  async set_attribute_system_status(system_status: Status): Promise<void> {
    this.status = system_status;
  }
  async get_attribute_version(): Promise<string> {
    return '1.0.0';
  }
}
const service = new MyAllScenarios();
const handler = createRouter(
  Object.values(AllScenariosServiceOperations),
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
