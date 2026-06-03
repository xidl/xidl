import { createServer } from 'node:http';
import type {
  RestServerGetKey1Response,
  RestServerGetKey2Response,
  RestServerGetKey3Response,
  RestServerGetKey4Response,
  RestServerGetKeyOptionsResponse,
  RestServerLoginRealmResponse,
  RestServerLoginResponse,
  Timestamp,
  UserInfo,
} from './rest_server.js';
import {
  createRestServerHandler,
  type RestServer,
} from './rest_server.server.js';

class MyRestServer implements RestServer {
  private host = 'localhost';
  private serverName = 'rest_server';
  private userInfo = new Map<number, UserInfo>();
  private keyStore = new Map<string, string>();

  async get_host(): Promise<string> {
    return this.host;
  }

  async set_host(req: { value: string }): Promise<void> {
    this.host = req.value;
  }

  async get_port(): Promise<number> {
    return 8081;
  }

  async get_server_name(): Promise<string> {
    return this.serverName;
  }

  async set_server_name(req: { name: string }): Promise<void> {
    this.serverName = req.name;
  }

  async get_user_info(req: { id: number }): Promise<UserInfo> {
    const user = this.userInfo.get(req.id);
    if (!user) {
      const { XidlServerError } = await import('./rest_server.server.js');
      throw new XidlServerError('Not Found', 404);
    }
    return user;
  }

  async query_user_info(req: { id: number }): Promise<UserInfo> {
    return this.get_user_info(req);
  }

  async post_user_info(req: { id: number; info: UserInfo }): Promise<void> {
    this.userInfo.set(req.id, req.info);
  }

  async put_key_value(req: { key: string; value: string }): Promise<void> {
    this.keyStore.set(req.key, req.value);
  }

  async delete_key(req: { key: string }): Promise<void> {
    this.keyStore.delete(req.key);
  }

  async patch_key(req: { key: string; value: string }): Promise<void> {
    this.keyStore.set(req.key, req.value);
  }

  async is_key_exists(req: { key_alias: string }): Promise<void> {
    if (!this.keyStore.has(req.key_alias)) {
      const { XidlServerError } = await import('./rest_server.server.js');
      throw new XidlServerError('Not Found', 404);
    }
  }

  async get_key_options(req: {
    key: string;
  }): Promise<RestServerGetKeyOptionsResponse> {
    return {
      exists: this.keyStore.has(req.key),
    };
  }

  async get_key_1(req: { key: string }): Promise<RestServerGetKey1Response> {
    const val = this.keyStore.get(req.key);
    if (val === undefined) {
      const { XidlServerError } = await import('./rest_server.server.js');
      throw new XidlServerError('Not Found', 404);
    }
    return { value: val };
  }

  async get_key_2(req: { key: string }): Promise<RestServerGetKey2Response> {
    return this.get_key_1(req);
  }

  async get_key_3(req: { key: string }): Promise<RestServerGetKey3Response> {
    return this.get_key_1(req);
  }

  async get_key_4(req: { key: string }): Promise<RestServerGetKey4Response> {
    return this.get_key_1(req);
  }

  async login(): Promise<RestServerLoginResponse> {
    return {
      session_id: 'simple_session_id',
    };
  }

  async login_realm(): Promise<RestServerLoginRealmResponse> {
    return {
      session_id: 'simple_session_id',
    };
  }

  async is_logined(req: { session_id: string }): Promise<boolean> {
    return req.session_id !== '';
  }

  async login_bearer(): Promise<void> {}

  async get_timestamp(): Promise<Timestamp> {
    throw new Error('unimplemented');
  }

  async is_admin(): Promise<void> {
    throw new Error('unimplemented');
  }
}

const handler = createRestServerHandler(new MyRestServer());

const port = process.env.PORT ? parseInt(process.env.PORT, 10) : 8080;
const server = createServer(async (req, res) => {
  try {
    const protocol = req.headers['x-forwarded-proto'] || 'http';
    const host = req.headers.host || 'localhost';
    const url = new URL(req.url || '', `${protocol}://${host}`);

    const chunks: Buffer[] = [];
    for await (const chunk of req) {
      chunks.push(chunk);
    }
    const body = chunks.length > 0 ? Buffer.concat(chunks) : undefined;

    const requestHeaders = new Headers();
    for (const [key, value] of Object.entries(req.headers)) {
      if (Array.isArray(value)) {
        for (const val of value) {
          requestHeaders.append(key, val);
        }
      } else if (value !== undefined) {
        requestHeaders.set(key, value as string);
      }
    }

    const request = new Request(url.toString(), {
      body: req.method !== 'GET' && req.method !== 'HEAD' ? body : undefined,
      headers: requestHeaders,
      method: req.method,
    });

    const response = await handler(request);

    res.statusCode = response.status;
    response.headers.forEach((val, key) => {
      res.setHeader(key, val);
    });

    if (response.body) {
      const reader = response.body.getReader();
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        res.write(value);
      }
    }
    res.end();
  } catch (err) {
    console.error('Handler error:', err);
    res.statusCode = 500;
    res.end('Internal Server Error');
  }
});

server.listen(port, '127.0.0.1', () => {
  console.log(`TS server starting on port ${port}`);
});
