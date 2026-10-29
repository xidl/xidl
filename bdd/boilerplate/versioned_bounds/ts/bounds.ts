import assert from 'node:assert/strict';
import * as a from './versioned_bounds.adapters';

type Version = 'v1' | 'v2' | 'v3';
const hooks: a.VersionAdapters = {};
let calls = 0;

function batch(version: Version, count: number) {
  const invoices = Array.from({ length: count }, (_, id) => ({
    id,
    ...(version !== 'v1' && { memo: 'payment received' }),
    ...(version === 'v3' && { label: 'approved' }),
  }));
  return {
    days: [invoices, []],
    invoices,
    regions: { eu: invoices, us: [] },
    retry: [],
  };
}

for (const version of ['v1', 'v2', 'v3'] as const) {
  for (let count = 0; count <= 2; count++) {
    const wire = batch(version, count);
    const result = await a.dispatchBatchesReserve(
      version,
      { batch: wire },
      async (request, context) => {
        calls++;
        assert.equal(context.originalVersion, version);
        assert.equal(request.batch.invoices.length, count);
        for (const invoice of request.batch.invoices) {
          if (version === 'v1') assert.equal(invoice.memo, undefined);
          if (version !== 'v3') assert.equal(invoice.label, undefined);
        }
        return { ok: true, value: request.batch };
      },
      hooks,
    );
    assert(result.dispatched && result.projected && result.value.ok);
    assert.deepEqual(JSON.parse(JSON.stringify(result.value.value)), wire);
  }
  for (const [field, value] of [
    ['invoices', [{ id: 1 }, { id: 2 }, { id: 3 }]],
    ['regions', { eu: [], jp: [], us: [] }],
    ['regions', { eu: [{ id: 1 }, { id: 2 }, { id: 3 }] }],
    ['days', [[{ id: 1 }, { id: 2 }, { id: 3 }], []]],
    ['days', [[]]],
    ['retry', [{ id: 1 }, { id: 2 }, { id: 3 }]],
    ['invoices', [{ id: '1' }]],
  ] as const) {
    const before = calls;
    const result = await a.dispatchBatchesReserve(
      version,
      { batch: { ...batch(version, 0), [field]: value } },
      async request => {
        calls++;
        return { ok: true, value: request.batch };
      },
      hooks,
    );
    assert.equal(result.dispatched, false, field);
    assert.equal(calls, before, 'invalid wire data must not reach the handler');
  }
}
for (const version of ['v1', 'v2', 'v3'] as const) {
  for (let count = 0; count <= 3; count++) {
    const before = calls;
    const result = await a.dispatchBatchesUpload(
      version,
      { bytes: new Uint8Array(count) },
      async request => {
        calls++;
        return { ok: true, value: request.bytes.byteLength };
      },
      hooks,
    );
    if (count > 2) {
      assert.equal(result.dispatched, false);
      assert.equal(calls, before, 'oversized bytes must not reach the handler');
    } else {
      assert(result.dispatched && result.projected && result.value.ok);
      assert.equal(result.value.value, count);
    }
  }
}
assert.equal(calls, 18);
console.log('PASS: bounded projections preserve three frozen contracts');
