import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createRouter } from 'xidl-typescript-server';
import { BatchesClient } from './bounded_sequences.client.js';
import { BatchRejected } from './bounded_sequences.errors.js';
import { BatchesOperations } from './bounded_sequences.server.js';

type Batch = Parameters<BatchesClient['reserve']>[0];
let accepted = 0;
const handler = createRouter(Object.values(BatchesOperations), {
  accepted() {
    return { count: accepted };
  },
  cookies(values: number[]) {
    return values;
  },
  download(raw: boolean, invalid: boolean) {
    const ids = invalid ? [1, 2, 3] : [1, 2];
    return raw
      ? { kind: 'OctetStream', value: new Uint8Array(ids) }
      : { kind: 'Json', value: ids };
  },
  exported(invalid: boolean) {
    const ids = invalid ? [1, 2, 3] : [1, 2];
    return { ids, tokens: ids };
  },
  headers(values: number[]) {
    return values;
  },
  metadata(
    invalid: boolean,
    groups: Record<string, number[]>,
    _backup: Record<string, number[]>,
  ) {
    return { value: invalid ? { eu: [1, 2, 3] } : groups };
  },
  oversized(): Batch {
    return {
      days: [[], []],
      invoices: [],
      regions: { eu: [1, 2, 3] },
      windows: [],
    };
  },
  query(values: number[]) {
    return values;
  },
  rejected(fault: number) {
    const values = (channel: number) =>
      fault === channel ? [1, 2, 3] : [1, 2];
    throw new BatchRejected({
      detail: { count: 2 },
      ids: values(1),
      regions: { eu: values(4) },
      retry: values(2),
      state: 'true',
      tag: '123',
      token: 'true',
      tokens: values(3),
    });
  },
  reserve(batch: Batch) {
    accepted += 1;
    return batch;
  },
  upload(bytes: Uint8Array) {
    return { count: bytes.length };
  },
});
const port = process.env.PORT ? Number.parseInt(process.env.PORT, 10) : 8080;
const server = createServer(async (request, response) => {
  const url = new URL(
    request.url ?? '/',
    `http://${request.headers.host ?? 'localhost'}`,
  );
  const webRequest = new Request(url, {
    body:
      request.method !== 'GET' && request.method !== 'HEAD'
        ? (request as unknown as BodyInit)
        : undefined,
    // @ts-expect-error Node.js fetch requires duplex for streamed request bodies.
    duplex: 'half',
    headers: request.headers as HeadersInit,
    method: request.method,
  });
  const webResponse = await handler(webRequest);
  response.statusCode = webResponse.status;
  webResponse.headers.forEach((value, name) => {
    if (name !== 'set-cookie') response.setHeader(name, value);
  });
  const cookies = webResponse.headers.getSetCookie();
  if (cookies.length) response.setHeader('set-cookie', cookies);
  if (webResponse.body) {
    for await (const chunk of webResponse.body) {
      response.write(chunk);
    }
  }
  response.end();
});

server.listen(0, '127.0.0.1', async () => {
  try {
    const address = server.address();
    assert.ok(address && typeof address !== 'string');
    const client = new BatchesClient(`http://127.0.0.1:${address.port}`);
    for (const values of [[], [1], [1, 2]]) {
      const returned = await client.reserve({
        days: [values, []],
        invoices: values,
        regions: { eu: values },
        retry: values,
        windows: [values],
      });
      assert.deepEqual(returned.invoices, values);
    }
    assert.deepEqual(await client.query([1]), [1]);
    assert.deepEqual(await client.headers([1, 2]), [1, 2]);
    assert.deepEqual(await client.cookies([1, 2]), [1, 2]);
    assert.equal((await client.upload(new Uint8Array([1, 2]))).count, 2);
    await assert.rejects(
      () => client.rejected(0),
      (error: unknown) => {
        assert.ok(error instanceof BatchRejected, String(error));
        assert.deepEqual(error.ids, [1, 2]);
        assert.deepEqual(error.retry, [1, 2]);
        assert.deepEqual(error.tokens, [1, 2]);
        assert.equal(error.tag, '123');
        assert.equal(error.token, 'true');
        assert.equal(error.state, 'true');
        return true;
      },
    );
    for (const fault of [1, 2, 3, 4]) {
      await assert.rejects(
        () => client.rejected(fault),
        (error: unknown) => {
          assert.ok(!(error instanceof BatchRejected));
          return true;
        },
      );
    }
    for (const run of [
      () => client.query([1, 2, 3]),
      () => client.headers([1, 2, 3]),
      () => client.cookies([1, 2, 3]),
      () => client.upload(new Uint8Array([1, 2, 3])),
      () => client.oversized(),
    ]) {
      await assert.rejects(run);
    }
    assert.equal(
      (await client.accepted()).count,
      3,
      'invalid wire data cannot mutate billing',
    );
    // Composite text metadata has a separate decoding contract; the Rust
    // SDK journey covers its wire-type generation regression.
    assert.deepEqual((await client.exported(false)).ids, [1, 2]);
    await assert.rejects(() => client.exported(true));
    const json = await client.download(false, false);
    assert.deepEqual(json, { kind: 'Json', value: [1, 2] });
    const bytes = await client.download(true, false);
    assert.deepEqual(bytes, {
      kind: 'OctetStream',
      value: new Uint8Array([1, 2]),
    });
    await assert.rejects(() => client.download(false, true));
    await assert.rejects(() => client.download(true, true));
    const malformed = new BatchesClient('http://provider.invalid', {
      fetch: async input => {
        const url = new URL(String(input));
        if (url.pathname === '/rejected') {
          const fault = Number(url.searchParams.get('fault'));
          const values = (channel: number) =>
            fault === channel ? [1, 2, 3] : [1, 2];
          const headers = new Headers({
            'content-type': 'application/json',
            detail: '{"count":2}',
            retry: JSON.stringify(values(2)),
            tag: '123',
          });
          for (const cookie of [
            `tokens=${JSON.stringify(values(3))}`,
            'state=true',
            'token=true',
          ])
            headers.append('set-cookie', cookie);
          return new Response(
            JSON.stringify({ ids: values(1), regions: { eu: values(4) } }),
            {
              headers,
              status: 409,
            },
          );
        }
        if (url.pathname === '/download') {
          const raw = url.searchParams.get('raw') === 'true';
          return new Response(raw ? new Uint8Array([1, 2, 3]) : '[1,2,3]', {
            headers: {
              'content-type': raw
                ? 'application/octet-stream'
                : 'application/json',
            },
          });
        }
        const badCookie = url.searchParams.get('invalid') === 'true';
        const headers = new Headers({ ids: badCookie ? '1,2' : '1,2,3' });
        for (const token of badCookie ? [1, 2, 3] : [1, 2])
          headers.append('set-cookie', `tokens=${token}`);
        return new Response(null, { headers, status: 204 });
      },
    });
    await assert.rejects(
      () => malformed.exported(false),
      'invalid response header',
    );
    for (const fault of [1, 2, 3, 4]) {
      await assert.rejects(
        () => malformed.rejected(fault),
        (error: unknown) => !(error instanceof BatchRejected),
        'invalid exception must not become a typed business error',
      );
    }
    await assert.rejects(
      () => malformed.exported(true),
      'invalid response cookie',
    );
    await assert.rejects(
      () => malformed.download(false, false),
      'invalid JSON representation',
    );
    await assert.rejects(
      () => malformed.download(true, false),
      'invalid binary representation',
    );
    accepted = 0;
    server.close(() => server.listen(port, '127.0.0.1'));
  } catch (error) {
    console.error(error);
    process.exit(1);
  }
});
