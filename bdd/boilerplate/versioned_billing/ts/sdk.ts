import assert from 'node:assert/strict';
import { v1 } from './v1/versioned_billing.client.js';
import { Conflict, Rejected } from './v1/versioned_billing.errors.js';
import { v1 as firstSchemas } from './v1/versioned_billing.iface.zod.js';
import { v2 } from './v2/versioned_billing.client.js';
import { v2 as currentSchemas } from './v2/versioned_billing.iface.zod.js';

// These schemas are generated independently from each pinned API contract.
for (const schemas of [firstSchemas, currentSchemas]) {
  for (const limit of [undefined, '', ' ', '\t', '0\n', '0x0', '0e0', '0.0']) {
    assert.equal(
      schemas.InvoicesListRequestSchema.safeParse({ limit, tenant: 'acme' })
        .success,
      false,
      `invalid page size ${JSON.stringify(limit)}`,
    );
  }
  for (const limit of [0, '0', '+0', '-0']) {
    assert(
      schemas.InvoicesListRequestSchema.parse({ limit, tenant: 'acme' })
        .limit === 0,
    );
  }
  assert.equal(
    schemas.InvoicesListRequestSchema.parse({ limit: '-1', tenant: 'acme' })
      .limit,
    -1,
  );
}

const base = process.argv[2];
assert(base);
const checkedFetch =
  (expected: string): typeof fetch =>
  async (input, init) => {
    const request = new Request(input, {
      ...init,
      signal: AbortSignal.timeout(5000),
    });
    assert.equal(request.headers.get('X-Billing-Version'), expected);
    const response = await fetch(request);
    assert.equal(response.headers.get('X-Billing-Version'), expected);
    return response;
  };
const first = new v1.InvoicesClient(base, {
  fetch: checkedFetch('2025-01-01'),
});
const current = new v2.InvoicesClient(base, {
  fetch: checkedFetch('2025-04-01'),
});
const before = await current.audit('acme');
const capture = {
  invoice_id: 'inv_ts',
  payment: { amount: 400, payment_intent: 'pi_ts_1' },
};
const [original, replay] = await Promise.all([
  first.pay(capture, 'ts-1', 'acme'),
  current.pay(capture, 'ts-1', 'acme'),
]);
assert.equal(original.payment_intent, 'pi_ts_1');
assert.equal(replay.payments.length, 1);
assert.equal((await current.audit('acme')).commits, before.commits + 1);
const next = {
  invoice_id: 'inv_ts',
  payment: { amount: 600, payment_intent: 'pi_ts_2' },
};
for (let retry = 0; retry < 2; retry++) {
  await assert.rejects(
    first.pay(next, 'ts-2', 'acme'),
    error =>
      error instanceof Rejected && error.type_path === 'Invoice.payment_intent',
  );
}
const committed = await current.pay(next, 'ts-2', 'acme');
assert.equal(committed.payments.length, 2);
assert.equal(committed.amount_paid, 1000);
await assert.rejects(first._get('inv_ts', 'acme'), Rejected);
assert.deepEqual(await first.pay(capture, 'ts-1', 'acme'), original);
assert.deepEqual(await current.pay(capture, 'ts-1', 'acme'), replay);
await assert.rejects(
  first.pay(
    { ...capture, payment: { ...capture.payment, amount: 401 } },
    'ts-1',
    'acme',
  ),
  Conflict,
);
assert.deepEqual(await first.list(0, 'acme'), []);
assert.equal((await current.audit('acme')).commits, before.commits + 2);
console.log(
  'PASS: independent TypeScript SDKs preserve frozen billing contracts',
);
