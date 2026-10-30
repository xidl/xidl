import assert from 'node:assert/strict';
import { LedgerClient } from './stream_lifecycle.client.js';
import { Conflict, Retired } from './stream_lifecycle.errors.js';

const client = new LedgerClient(process.argv[2]);
async function collect<T>(source: AsyncIterable<T>): Promise<T[]> {
  const values: T[] = [];
  for await (const value of source) values.push(value);
  return values;
}
assert.deepEqual(await collect(client.export_rows('ready')), [
  { id: 'zero', quantity: 0 },
  { id: '雪', quantity: 2 },
]);
await assert.rejects(
  collect(client.export_rows('locked')),
  error => error instanceof Conflict && error.detail.job === 'locked',
);
await assert.rejects(collect(client.export_rows('retired')), Retired);
const partial = client.export_rows('partial')[Symbol.asyncIterator]();
assert.equal((await partial.next()).value.quantity, 0);
await assert.rejects(
  partial.next(),
  error => error instanceof Conflict && error.detail.job === 'partial',
);
assert.equal((await partial.next()).done, true);
assert.deepEqual(await collect(client.export_rows('empty')), []);
for await (const row of client.export_rows('ready')) {
  assert.equal(row.quantity, 0);
  break;
}
const input = async function* () {
  yield { row: { id: 'zero', quantity: 0 } };
  yield { row: { id: '雪', quantity: 2 } };
};
assert.deepEqual(await client.import_rows(input()), { accepted: 2, total: 2 });
const duplicate = async function* () {
  for (let i = 0; i < 2; i++) yield { row: { id: 'duplicate', quantity: 1 } };
};
await assert.rejects(
  client.import_rows(duplicate()),
  error => error instanceof Conflict && error.detail.job === 'duplicate',
);
