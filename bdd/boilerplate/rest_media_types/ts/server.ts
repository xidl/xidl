import { createServer } from 'node:http';
import type {
  RestMediaTypesApiGetMsgpackUserResponse,
  RestMediaTypesApiSubmitProfileResponse,
} from './rest_media_types.js';
import {
  createRestMediaTypesApiHandler,
  type RestMediaTypesApi,
} from './rest_media_types.server.js';

class MyRestMediaTypesService implements RestMediaTypesApi {
  async submit_profile(req: {
    name: string;
    age: number;
  }): Promise<RestMediaTypesApiSubmitProfileResponse> {
    return {
      normalized_name: req.name.toUpperCase(),
      return: `${req.name}:${req.age}`,
    };
  }

  async get_msgpack_user(req: {
    user_id: string;
  }): Promise<RestMediaTypesApiGetMsgpackUserResponse> {
    return {
      return: `user:${req.user_id}`,
      score: 95,
    };
  }
}

const handler = createRestMediaTypesApiHandler(new MyRestMediaTypesService());

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
