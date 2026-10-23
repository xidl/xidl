import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import type * as types from './versioned_adapters_business';
import * as a from './versioned_adapters_business.adapters';
import * as clients from './versioned_adapters_business.client';

export async function runHttpJourney(
  hooks: a.VersionAdapters,
  storedValues: a.StoredValues,
  wireParcel: Record<string, unknown>,
  head: types.v3.Parcel,
  getHookCalls: () => number,
): Promise<void> {
  // The application owns routing and idempotency. Dispatch invokes one Head handler.
  let writes = 0;
  const ledger = new Map<
    string,
    {
      fingerprint: string;
      outcome: a.OperationOutcome<types.v3.State, a.FrameworkError>;
    }
  >();
  const contexts: a.VersionContext[] = [];
  const server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url ?? '/', 'http://localhost');
      const [version, operation] = url.pathname.slice(1).split('/');
      if (version !== 'v1' && version !== 'v2' && version !== 'v3') {
        response.writeHead(404).end();
        return;
      }
      if (operation !== 'parcels' || request.method !== 'POST') {
        response.writeHead(404).end();
        return;
      }
      const chunks: Uint8Array[] = [];
      for await (const chunk of request) chunks.push(chunk);
      const parcel: unknown = JSON.parse(Buffer.concat(chunks).toString());
      const key = `${version}:${request.headers['idempotency-key']}`;
      let conflict = false;
      const result = await a.dispatchDepotShip(
        version,
        { parcel },
        async (input, context) => {
          contexts.push(context);
          const fingerprint = JSON.stringify(input);
          const cached = ledger.get(key);
          if (cached) {
            conflict = cached.fingerprint !== fingerprint;
            return cached.outcome;
          }
          writes++;
          const value =
            input.parcel.tracking === 'special'
              ? ('shipping' as const)
              : input.parcel.tracking === 'terminal'
                ? ('delivered' as const)
                : ('pending' as const);
          const outcome = { ok: true as const, value };
          ledger.set(key, { fingerprint, outcome });
          return outcome;
        },
        hooks,
      );
      response.setHeader('content-type', 'application/json');
      if (conflict) {
        response
          .writeHead(409)
          .end(JSON.stringify({ reason: 'idempotency key conflicts' }));
        return;
      }
      if (!result.dispatched) {
        response.writeHead(400).end(JSON.stringify(result.error));
        return;
      }
      if (!result.projected) {
        assert.equal(result.head, ledger.get(key)?.outcome);
        response
          .writeHead(422)
          .end(JSON.stringify({ reason: result.error.reason }));
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
  });
  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
  try {
    const base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
    const send = (version: string, parcel: unknown, key: string) =>
      fetch(`${base}/${version}/parcels`, {
        body: JSON.stringify(parcel),
        headers: { 'content-type': 'application/json', 'idempotency-key': key },
        method: 'POST',
      });
    const first = await send('v1', wireParcel, 'normal');
    assert.equal(first.status, 200);
    assert.equal(await first.json(), 'pending');
    const replay = await send('v1', wireParcel, 'normal');
    assert.equal(replay.status, 200);
    assert.equal(writes, 1);
    const conflict = await send(
      'v1',
      { ...wireParcel, reference: 'DIFFERENT' },
      'normal',
    );
    assert.equal(conflict.status, 409);
    assert.equal(writes, 1);
    const callsBeforeNetworkInvalid = getHookCalls();
    const bad = await send(
      'v1',
      { ...wireParcel, stops: [{ zip_code: 'x', zone: 'future' }] },
      'invalid',
    );
    assert.equal(bad.status, 400);
    assert.equal(writes, 1);
    assert.equal(getHookCalls(), callsBeforeNetworkInvalid);
    const unsupported = await send(
      'v1',
      { ...wireParcel, reference: 'special' },
      'committed',
    );
    assert.equal(unsupported.status, 422);
    assert.equal(writes, 2);
    assert.equal(
      (await send('v1', { ...wireParcel, reference: 'special' }, 'committed'))
        .status,
      422,
    );
    assert.equal(
      writes,
      2,
      'replaying a projection refusal never repeats a committed write',
    );
    const v2Wire = { ...wireParcel, carrier: 'postal' };
    const v3Wire = { ...v2Wire, audit: 'checked' };
    assert.equal(
      (await send('v2', { ...v2Wire, reference: 'special' }, 'v2')).status,
      200,
    );
    assert.equal(
      (await send('v2', { ...v2Wire, reference: 'terminal' }, 'middle')).status,
      200,
    );
    assert.equal(
      (
        await send(
          'v1',
          { ...wireParcel, reference: 'terminal' },
          'later-refusal',
        )
      ).status,
      422,
    );
    assert.deepEqual(ledger.get('v1:later-refusal')?.outcome, {
      ok: true,
      value: 'delivered',
    });
    assert.equal(
      (await send('v3', { ...v3Wire, reference: 'terminal' }, 'head')).status,
      200,
    );
    assert.deepEqual(contexts[0], {
      originalVersion: '2026-01',
      originalVersionId: 'v1',
    });
    const client = new clients.v1.DepotClient(`${base}/v1`, {
      headers: { 'idempotency-key': 'client' },
    });
    const v1Result = a.downcastParcel('v1', head, storedValues, hooks);
    assert(v1Result.ok);
    const v1Parcel = v1Result.value;
    assert.equal(
      await client.ship(v1Parcel),
      'pending',
      'a generated SDK must round-trip over HTTP',
    );
    console.log(
      'PASS: three-version wire validation, typed hooks, full error downgrade, HTTP SDK, idempotent committed outcomes',
    );
  } finally {
    server.closeAllConnections();
    await new Promise<void>((resolve, reject) =>
      server.close(error => (error ? reject(error) : resolve())),
    );
  }
}
