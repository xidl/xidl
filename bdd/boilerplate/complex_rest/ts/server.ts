import { createServer } from 'node:http';
import { createRouter, XidlServerError } from 'xidl-typescript-server';
import type { User } from './complex_rest.js';
import {
  type UserService,
  UserServiceOperations,
} from './complex_rest.server.js';

class MyUserService implements UserService {
  private users = new Map<number, User>();

  async get_user(id: number): Promise<User> {
    const user = this.users.get(id);
    if (!user) {
      throw new XidlServerError(404, 'Not Found');
    }
    return user;
  }

  async create_user(user: User): Promise<User> {
    this.users.set(user.id, user);
    return user;
  }

  async list_users(filter: string): Promise<User[]> {
    const result: User[] = [];
    for (const user of this.users.values()) {
      if (
        !filter ||
        user.roles.includes(filter) ||
        user.name.includes(filter)
      ) {
        result.push(user);
      }
    }
    return result;
  }
}

const service = new MyUserService();
const handler = createRouter(Object.values(UserServiceOperations), service);

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
