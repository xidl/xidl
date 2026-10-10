import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { Readable } from 'node:stream';
import { createRouter, XidlServerError } from 'xidl-typescript-server';
import { FilesClient } from './http_exceptions.client.js';
import {
  NotFound,
  NotModified,
  PreconditionFailed,
} from './http_exceptions.errors.js';
import { type Files, FilesOperations } from './http_exceptions.server.js';

const service: Files = {
  get_file(id) {
    switch (id) {
      case 'cached':
        throw new NotModified({ etag: '"v2"' });
      case 'stale':
      case 'stale-minimal': {
        const full = id === 'stale';
        throw new PreconditionFailed({
          attempts: full ? ['first', 'second'] : undefined,
          code: 41201,
          detail: full ? 'reload before retry' : undefined,
          etag: '"v2"',
          hints: full ? ['reload', 'retry'] : undefined,
          msg: 'stale revision',
          retry_token: full ? '001/;%' : undefined,
          session_id: 'renewed',
        });
      }
      case 'missing':
        throw new NotFound({ code: 40401, msg: 'not found' });
      case 'framework-typed':
        throw new XidlServerError(412, 'framework conflict');
      case 'framework':
        throw new XidlServerError(500, 'storage unavailable');
      default:
        return { etag: 'v2', id };
    }
  },
};
const handler = createRouter(Object.values(FilesOperations), service);

const server = createServer(async (req, res) => {
  const response = await handler(
    new Request(`http://${req.headers.host}${req.url}`),
  );
  res.statusCode = response.status;
  for (const [name, value] of response.headers) {
    if (name !== 'set-cookie') res.setHeader(name, value);
  }
  res.setHeader('Set-Cookie', response.headers.getSetCookie());
  if (response.body) Readable.fromWeb(response.body).pipe(res);
  else res.end();
});
const port = Number(process.env.PORT ?? 8080);
server.listen(0, '127.0.0.1', async () => {
  try {
    const address = server.address();
    assert.ok(address && typeof address !== 'string');
    const client = new FilesClient(`http://127.0.0.1:${address.port}`);
    assert.equal((await client.get_file('ok')).id, 'ok');
    await assert.rejects(client.get_file('cached'), (error: unknown) => {
      assert.ok(error instanceof NotModified);
      assert.equal(error.etag, '"v2"');
      return true;
    });
    for (const full of [true, false]) {
      await assert.rejects(
        client.get_file(full ? 'stale' : 'stale-minimal'),
        (error: unknown) => {
          assert.ok(error instanceof PreconditionFailed);
          assert.equal(error.etag, '"v2"');
          assert.equal(error.session_id, 'renewed');
          assert.equal(error.retry_token, full ? '001/;%' : undefined);
          assert.deepEqual(error.hints, full ? ['reload', 'retry'] : undefined);
          assert.deepEqual(
            error.attempts,
            full ? ['first', 'second'] : undefined,
          );
          assert.equal(error.msg, 'stale revision');
          assert.equal(error.code, 41201);
          assert.equal(error.detail, full ? 'reload before retry' : undefined);
          return true;
        },
      );
    }
    await assert.rejects(client.get_file('missing'), NotFound);
    await assert.rejects(client.get_file('framework'), { code: 500 });
    await assert.rejects(client.get_file('framework-typed'), {
      code: 412,
      status: 412,
    });

    console.log('generated client exception round trips passed');
    server.close(() => server.listen(port, '127.0.0.1'));
  } catch (error) {
    console.error(error);
    process.exit(1);
  }
});
