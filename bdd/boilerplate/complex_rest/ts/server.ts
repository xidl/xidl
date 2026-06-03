import { createServer } from 'node:http';
import type { User } from './complex_rest.js';
import {
  createUserServiceHandler,
  type UserService,
} from './complex_rest.server.js';

class MyUserService implements UserService {
  private users = new Map<number, User>();

  async get_user(req: { id: number }): Promise<User> {
    const user = this.users.get(req.id);
    if (!user) {
      const { XidlServerError } = await import('./complex_rest.server.js');
      throw new XidlServerError('Not Found', 404);
    }
    return user;
  }

  async create_user(req: { user: User }): Promise<User> {
    this.users.set(req.user.id, req.user);
    return req.user;
  }

  async list_users(req: { filter: string }): Promise<User[]> {
    const result: User[] = [];
    for (const user of this.users.values()) {
      if (
        !req.filter ||
        user.roles.includes(req.filter) ||
        user.name.includes(req.filter)
      ) {
        result.push(user);
      }
    }
    return result;
  }
}

const handler = createUserServiceHandler(new MyUserService());

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
