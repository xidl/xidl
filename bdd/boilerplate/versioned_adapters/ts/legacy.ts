import { execFile } from 'node:child_process';
import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import { join } from 'node:path';
import { promisify } from 'node:util';
import { serialize } from 'xidl-typescript-codec';
import * as a from './versioned_adapters.adapters';
import * as errors from './versioned_adapters.errors';
import * as schemas from './versioned_adapters.zod';

const ok = <T>(value: T): a.VersionedResult<T> => ({ ok: true, value });
const hooks: a.VersionAdapters = {
  itemDowncastToV1: source => ok({ ref_v1: source.ref_v2 }),
  itemsCreateRequestUpcastFromV1: () => ok({ actor: 'adapter' }),
  itemsGetLockedErrorDowncastToV1: source =>
    ok(new errors.v1$NotFound({ id: source.owner })),
  itemUpcastFromV1: source =>
    ok({
      color: a.IGNORED,
      owner: 'ops',
      ref_v1: a.IGNORED,
      ref_v2: source.ref_v1,
    }),
  stateDowncastToV1: () => ({
    error: {
      direction: 'downcast',
      reason: 'archived is unavailable in v1',
      typePath: 'State',
      version: '1.0',
    },
    ok: false,
  }),
};
const headGet: Parameters<typeof a.dispatchItemsGet>[2] = async ({ id }) =>
  id === 'locked'
    ? { error: new errors.Locked({ owner: 'ops' }), ok: false }
    : {
        ok: true,
        value: {
          name: id,
          note: 'head note',
          owner: 'ops',
          ref_v2: `ref-${id}`,
        },
      };
const headCreate: Parameters<typeof a.dispatchItemsCreate>[2] = async ({
  item,
}) => ({ ok: true, value: item.ref_v2 === 'special' ? 'archived' : 'draft' });
const servers = ['v1', 'v2'].map(version =>
  createServer(async (request, response) => {
    try {
      const resolved = version === 'v1' ? 'v1' : 'v2';
      const url = new URL(request.url ?? '/', 'http://localhost');
      response.setHeader('content-type', 'application/json');
      if (request.method === 'GET') {
        const result = await a.dispatchItemsGet(
          resolved,
          { id: url.pathname.split('/').at(-1) },
          headGet,
          hooks,
        );
        if (!result.dispatched) {
          response.writeHead(400).end(JSON.stringify(result.error));
          return;
        }
        if (!result.projected) {
          response.writeHead(500).end(JSON.stringify(result.error));
          return;
        }
        if (!result.value.ok) {
          const error = result.value.error;
          response
            .writeHead(error.kind === 'Framework' ? 500 : error.status)
            .end(JSON.stringify(error));
          return;
        }
        const schema =
          version === 'v1' ? schemas.v1.ItemSchema : schemas.v2.ItemSchema;
        response
          .writeHead(200)
          .end(JSON.stringify(serialize(result.value.value, schema)));
        return;
      }
      const chunks: Uint8Array[] = [];
      for await (const chunk of request) chunks.push(chunk);
      const item: unknown = JSON.parse(Buffer.concat(chunks).toString());
      const raw = {
        item,
        ...(url.searchParams.has('actor')
          ? { actor: url.searchParams.get('actor') }
          : {}),
      };
      const result = await a.dispatchItemsCreate(
        resolved,
        raw,
        headCreate,
        hooks,
      );
      if (!result.dispatched) {
        response.writeHead(400).end(JSON.stringify(result.error));
        return;
      }
      if (!result.projected) {
        response.writeHead(500).end(JSON.stringify(result.error));
        return;
      }
      if (!result.value.ok) {
        response.writeHead(500).end(JSON.stringify(result.value.error));
        return;
      }
      response.writeHead(200).end(JSON.stringify(result.value.value));
    } catch (error) {
      response.writeHead(500).end(String(error));
    }
  }),
);
try {
  for (const [index, server] of servers.entries()) {
    await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
    const port = (server.address() as AddressInfo).port;
    const file = join(process.argv[2], `versioned_adapters.v${index + 1}.hurl`);
    await promisify(execFile)('hurl', [
      '--test',
      '--variable',
      `base_url=http://127.0.0.1:${port}`,
      file,
    ]);
  }
  console.log(
    'PASS: unchanged versioned_adapters.v1/v2 Hurl contracts through independent TS target and one Head implementation',
  );
} finally {
  for (const server of servers) {
    if (!server.listening) continue;
    server.closeAllConnections();
    await new Promise<void>((resolve, reject) =>
      server.close(error => (error ? reject(error) : resolve())),
    );
  }
}
