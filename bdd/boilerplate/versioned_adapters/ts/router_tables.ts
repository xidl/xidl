import assert from 'node:assert/strict';
import { createVersionedAdapterRouter } from 'xidl-typescript-server';
import {
  bindVersionedInterfaces,
  VERSIONS,
} from './versioned_adapter_tables.adapters.js';
import { v1, v2 } from './versioned_adapter_tables.server.js';

const sources = {
  v1: { id: 'v1', tables: v1 },
  v2: { id: 'v2', tables: v2 },
} as const;

const failure = new Error('item lookup failed');
const items: Parameters<typeof bindVersionedInterfaces.Items>[1] = {
  async _get({ id }, context) {
    if (id === 'broken') throw failure;
    return {
      ok: true,
      value: { id: `item:${id}`, label: context.originalVersionId },
    };
  },
};

const accounts: Parameters<typeof bindVersionedInterfaces.Accounts>[1] = {
  async _get({ id }) {
    return { ok: true, value: { id: `account:${id}` } };
  },
  async summary() {
    return { ok: true, value: 'ready' };
  },
};

export function checkInvalidBindings(): void {
  const wrongTable = { id: 'v1', tables: v1.AccountsOperations } as const;
  const incomplete = { id: 'v1', tables: { ItemsOperations: {} } } as const;
  // @ts-expect-error The version module must contain ItemsOperations.
  bindVersionedInterfaces.Items({ id: 'v1', tables: {} }, items, {});
  // @ts-expect-error An Accounts table cannot stand in for the Items module.
  bindVersionedInterfaces.Items(wrongTable, items, {});
  // @ts-expect-error An operation present in the projected contract requires a descriptor.
  bindVersionedInterfaces.Items(incomplete, items, {});
  // @ts-expect-error Every mounted operation needs its Head method.
  bindVersionedInterfaces.Accounts(sources.v2, { _get: accounts._get }, {});
}

const route = createVersionedAdapterRouter({
  parameter: { in: 'header', name: 'X-Version' },
  routerOptions: {
    authorize(_request, _requirements, { operation }) {
      const expected =
        operation.path === '/account-summary' ? 'summary' : '_get';
      assert.equal(operation.handler, expected);
    },
  },
  versions: VERSIONS.map(({ id, value }) => ({
    id,
    interfaces: [
      bindVersionedInterfaces.Items(sources[id], items, {}),
      bindVersionedInterfaces.Accounts(sources[id], accounts, {}),
    ],
    value,
  })),
});

for (const { id, value } of VERSIONS) {
  const request = (path: string, method = 'GET') =>
    new Request(`http://localhost${path}`, {
      headers: { 'X-Version': value },
      method,
    });

  const item = await route(request('/items/42'));
  assert.equal(item.status, 200);
  assert.equal(item.headers.get('X-Version'), value);
  assert.equal(item.headers.get('Vary'), 'X-Version');
  assert.deepEqual(
    await item.json(),
    id === 'v1' ? { id: 'item:42' } : { id: 'item:42', label: 'v2' },
  );

  const account = await route(request('/accounts/42'));
  assert.equal(account.status, 200);
  assert.deepEqual(await account.json(), { id: 'account:42' });

  const summary = await route(request('/account-summary'));
  assert.equal(summary.status, id === 'v1' ? 404 : 200);
  if (id === 'v2') assert.equal(await summary.text(), 'ready');

  const wrongMethod = await route(request('/items/42', 'POST'));
  assert.equal(wrongMethod.status, 405);
  assert.equal(wrongMethod.headers.get('Allow'), 'GET');
  await assert.rejects(
    route(request('/items/broken')),
    error => error === failure,
  );
}

const first = bindVersionedInterfaces.Items(sources.v1, items, {});
assert.throws(
  () =>
    createVersionedAdapterRouter({
      parameter: { in: 'header', name: 'X-Version' },
      versions: [{ id: 'v1', interfaces: [first, first], value: '2026-12-01' }],
    }),
  /conflicting GET route .* across operation tables/,
);

const latest = bindVersionedInterfaces.Accounts(sources.v2, accounts, {});
const summary = latest.operations.find(
  ({ operation }) => operation.handler === 'summary',
);
assert(summary);
// @ts-expect-error The bound operation is available only in v2.
const unsupported = await summary.binding.dispatch('v1', {});
assert.equal(unsupported.dispatched, false);
console.log('PASS: generated adapter tables');
