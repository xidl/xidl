import assert from 'node:assert/strict';
import { StoreClient } from './wire_probe.client.js';
import { Conflict } from './wire_probe.errors.js';
import type { catalog } from './wire_probe.js';

const client = new StoreClient(process.argv[2]);
const item: catalog.Item = {
  availability: 'inStock',
  initial: 'A',
  letters: ['A', '雪', '🚀'],
  mark: '🚀',
  revision: 9007199254740991,
  sku: 'sku-雪',
  stock_grid: [
    [0, 1, 2],
    [3, 4, 5],
  ],
  thumbnail: [0, 127, 255],
};
assert.deepEqual(await client.echo(item), item);
await assert.rejects(
  client.echo({ ...item, sku: 'locked' }),
  error => error instanceof Conflict && error.sku === 'locked',
);
for (const change of [
  { data: item, tag: '0' },
  { data: item, tag: '1' },
  { data: [item], tag: '2' },
  { data: [], tag: '2' },
  { data: 'retired', tag: 'default' },
] satisfies catalog.Change[]) {
  assert.deepEqual(await client.apply(change), change);
}
const bindings = await client.bindings(7, 'A', '%2F+ %');
for (const tag of ["'\"'", "'\\\\'", "'\\n'", "'A'", 'default'] as const) {
  const label: catalog.LabelChange = { data: 'retired', tag };
  assert.deepEqual(await client.label(label), label);
}
assert.deepEqual(bindings, {
  count_reply: 7,
  receipt: '%2F+ %',
  return: '7:A:%2F+ %',
});
const values: catalog.Change[] = [];
for await (const value of client.feed('ready')) values.push(value);
assert.deepEqual(values, [
  { data: item, tag: '0' },
  { data: 'retired', tag: 'default' },
]);
const stream = client.feed('partial')[Symbol.asyncIterator]();
assert.deepEqual((await stream.next()).value, { data: item, tag: '0' });
await assert.rejects(stream.next(), Conflict);
assert.equal((await stream.next()).done, true);
