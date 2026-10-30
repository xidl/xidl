import assert from 'node:assert/strict';
import test from 'node:test';
import { z } from 'zod';
import { deserialize, serialize, xjson } from '../src/index.js';

test('union branches retain nested wire metadata in objects and collections', () => {
  const item = z.object({
    note: xjson(z.string().optional(), { ignore: true }),
    sku: xjson(z.string(), { name: 'sku_id' }),
  });
  const schema = z.union([
    z.object({ data: item, tag: z.enum(['0', '1']) }),
    z.object({ data: z.array(item), tag: z.literal('2') }),
    z.object({ data: z.string(), tag: z.literal('default') }),
  ]);
  for (const [model, wire] of [
    [
      { data: { sku: '雪' }, tag: '0' },
      { data: { sku_id: '雪' }, tag: '0' },
    ],
    [
      { data: { sku: '雪' }, tag: '1' },
      { data: { sku_id: '雪' }, tag: '1' },
    ],
    [
      { data: [{ sku: '雪' }], tag: '2' },
      { data: [{ sku_id: '雪' }], tag: '2' },
    ],
    [
      { data: [], tag: '2' },
      { data: [], tag: '2' },
    ],
    [
      { data: 'retired', tag: 'default' },
      { data: 'retired', tag: 'default' },
    ],
  ]) {
    assert.deepEqual(serialize(schema.parse(model), schema), wire);
    assert.deepEqual(schema.parse(deserialize(wire, schema)), model);
  }
  assert.deepEqual(
    serialize({ data: { note: 'private', sku: '雪' }, tag: '1' }, schema),
    { data: { sku_id: '雪' }, tag: '1' },
  );
  for (const value of [
    { data: { sku_id: '雪' }, tag: '3' },
    { data: null, tag: '1' },
    { data: null, tag: '2' },
    { data: { sku_id: 1 }, tag: '1' },
    { data: { sku: 'internal model field' }, tag: '1' },
    { data: {}, tag: 'default' },
  ])
    assert.throws(() => schema.parse(deserialize(value, schema)));
});

test('an invalid nested branch does not prevent a later union alternative', () => {
  const schema = z.union([
    z.object({ value: z.union([z.literal('a'), z.literal('b')]) }),
    z.object({ value: xjson(z.number(), { name: 'count' }) }),
  ]);
  assert.deepEqual(schema.parse(deserialize({ count: 3 }, schema)), {
    value: 3,
  });
  assert.throws(() => deserialize({ value: 3 }, schema));
});
