import assert from 'node:assert/strict';
import test from 'node:test';

import { z } from 'zod';

import { defineOperation, XidlServerError } from '../src/index.js';
import { createNextRoute } from '../src/next.js';

interface UserServer {
  createUser(request: {
    id: number;
    name: string;
  }): Promise<{ id: number; name: string }>;
  getUser(request: { id: number }): Promise<{ id: number; name: string }>;
}

const userSchema = z.object({ id: z.number(), name: z.string() });
const getUser = defineOperation<UserServer, 'getUser'>({
  handler: 'getUser',
  method: 'GET',
  path: '/users/{id}',
  request: {
    body: { contentType: '', fields: [], kind: 'none' },
    cookies: [],
    headers: [],
    kind: 'object',
    path: [
      {
        catchAll: false,
        key: 'id',
        multi: false,
        parameter: 'id',
        wireName: 'id',
      },
    ],
    query: [],
    schema: z.object({ id: z.coerce.number() }),
  },
  response: {
    bodyFields: [{ key: 'return', wireName: 'return' }],
    bodyMode: 'return',
    contentType: 'application/json',
    cookies: [],
    headers: [],
    stream: false,
  },
  security: [],
});

const createUser = defineOperation<UserServer, 'createUser'>({
  handler: 'createUser',
  method: 'POST',
  path: '/users',
  request: {
    body: {
      contentType: 'application/json',
      fields: [
        { key: 'id', wireName: 'id' },
        { key: 'name', wireName: 'name' },
      ],
      kind: 'value',
      schema: userSchema,
    },
    cookies: [],
    headers: [],
    kind: 'object',
    path: [],
    query: [],
    schema: userSchema,
  },
  response: {
    bodyFields: [{ key: 'return', wireName: 'return' }],
    bodyMode: 'return',
    contentType: 'application/json',
    cookies: [],
    headers: [],
    stream: false,
  },
  security: [],
});

test('createNextRoute decodes Next.js params and returns JSON', async () => {
  const server: UserServer = {
    async createUser(request) {
      return request;
    },
    async getUser(request) {
      return { id: request.id, name: 'Alice' };
    },
  };
  const handler = createNextRoute(getUser, server);
  const response = await handler(new Request('http://localhost/users/7'), {
    params: Promise.resolve({ id: '7' }),
  });

  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { id: 7, name: 'Alice' });
});

test('createNextRoute decodes JSON request bodies', async () => {
  const server: UserServer = {
    async createUser(request) {
      return request;
    },
    async getUser() {
      throw new XidlServerError('not found', 404);
    },
  };
  const handler = createNextRoute(createUser, server);
  const response = await handler(
    new Request('http://localhost/users', {
      body: JSON.stringify({ id: 3, name: 'Bob' }),
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
    }),
    { params: Promise.resolve({}) },
  );

  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { id: 3, name: 'Bob' });
});

test('createNextRoute preserves numeric-looking string params', async () => {
  interface StringIdServer {
    getItem(request: { id: string }): { id: string };
  }
  const operation = defineOperation<StringIdServer, 'getItem'>({
    ...getUser,
    handler: 'getItem',
    request: {
      ...getUser.request,
      schema: z.object({ id: z.coerce.string() }),
    },
  });
  const handler = createNextRoute(operation, {
    getItem: request => request,
  });

  const response = await handler(new Request('http://localhost/items/001'), {
    params: Promise.resolve({ id: '001' }),
  });

  assert.deepEqual(await response.json(), { id: '001' });
});

test('createNextRoute maps body wire names to service keys', async () => {
  interface AliasServer {
    createItem(request: { displayName: string }): { displayName: string };
  }
  const schema = z.object({ displayName: z.string() });
  const operation = defineOperation<AliasServer, 'createItem'>({
    handler: 'createItem',
    method: 'POST',
    path: '/items',
    request: {
      body: {
        contentType: 'application/json',
        fields: [{ key: 'displayName', wireName: 'display_name' }],
        kind: 'value',
        schema,
      },
      cookies: [],
      headers: [],
      kind: 'object',
      path: [],
      query: [],
      schema,
    },
    response: {
      bodyFields: [{ key: 'return', wireName: 'return' }],
      bodyMode: 'return',
      contentType: 'application/json',
      cookies: [],
      headers: [],
      stream: false,
    },
    security: [],
  });
  const handler = createNextRoute(operation, {
    createItem: request => request,
  });

  const response = await handler(
    new Request('http://localhost/items', {
      body: JSON.stringify({ display_name: 'Alice' }),
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
    }),
    { params: Promise.resolve({}) },
  );

  assert.deepEqual(await response.json(), { displayName: 'Alice' });
});

test('createNextRoute passes the response schema to custom codecs', async () => {
  let receivedSchema: unknown;
  const operation = defineOperation<UserServer, 'createUser'>({
    ...createUser,
    response: { ...createUser.response, schema: userSchema },
  });
  const handler = createNextRoute(
    operation,
    {
      createUser: async request => request,
      getUser: async request => ({ id: request.id, name: 'Alice' }),
    },
    {
      codecs: {
        'application/json': {
          encode(value, schema) {
            receivedSchema = schema;
            return JSON.stringify(value);
          },
        },
      },
    },
  );

  await handler(
    new Request('http://localhost/users', {
      body: JSON.stringify({ id: 3, name: 'Bob' }),
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
    }),
    { params: Promise.resolve({}) },
  );

  assert.equal(receivedSchema, userSchema);
});

test('createNextRoute maps typed server errors', async () => {
  const server: UserServer = {
    async createUser(request) {
      return request;
    },
    async getUser() {
      throw new XidlServerError('not found', 404);
    },
  };
  const handler = createNextRoute(getUser, server);
  const response = await handler(new Request('http://localhost/users/9'), {
    params: Promise.resolve({ id: '9' }),
  });

  assert.equal(response.status, 404);
  assert.deepEqual(await response.json(), {
    code: 404,
    msg: 'not found',
  });
});

test('createNextRoute hides unexpected server errors', async () => {
  const server: UserServer = {
    async createUser(request) {
      return request;
    },
    async getUser() {
      throw new Error('database password leaked');
    },
  };
  const handler = createNextRoute(getUser, server);
  const response = await handler(new Request('http://localhost/users/9'), {
    params: Promise.resolve({ id: '9' }),
  });

  assert.equal(response.status, 500);
  assert.deepEqual(await response.json(), {
    code: 500,
    msg: 'internal server error',
  });
});
