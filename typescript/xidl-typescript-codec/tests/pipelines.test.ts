import assert from 'node:assert/strict';
import { test } from 'node:test';
import { z } from 'zod';
import { deserialize, serialize, xjson } from '../src/index.js';

test('numeric pipelines retain zero omission and leave input validation to Zod', () => {
  const numeric = z
    .union([z.number(), z.string().min(1).transform(Number)])
    .pipe(z.number());
  const integer = numeric.pipe(z.number().int());
  const schema = z.object({
    alias: xjson(integer, { omitempty: true }),
    count: xjson(integer, { name: 'wire_count', omitempty: true }),
    ratio: xjson(numeric, { omitempty: true }),
  });

  assert.deepEqual(serialize({ alias: 0, count: 0, ratio: 0 }, schema), {});
  assert.deepEqual(serialize({ alias: 2, count: 1, ratio: 0.5 }, schema), {
    alias: 2,
    ratio: 0.5,
    wire_count: 1,
  });

  const invalid = deserialize({ alias: 0, ratio: 0, wire_count: '' }, schema);
  assert.equal(invalid.count, '');
  assert.equal(schema.safeParse(invalid).success, false);
  assert.deepEqual(
    schema.parse(
      deserialize({ alias: '2', ratio: '0.5', wire_count: '0' }, schema),
    ),
    { alias: 2, count: 0, ratio: 0.5 },
  );
});

test('object pipelines use input fields to decode and output fields to encode', () => {
  const conversion = z
    .object({ raw: xjson(z.string(), { name: 'wire_count' }) })
    .transform(({ raw }) => ({ count: Number(raw) }))
    .pipe(z.object({ count: xjson(z.number(), { name: 'units' }) }));
  const schema = z.object({
    entries: z.array(conversion).optional(),
    flattened: xjson(conversion, { flatten: true }),
  });
  const input = { entries: [{ wire_count: '3' }], wire_count: '2' };
  const model = { entries: [{ count: 3 }], flattened: { count: 2 } };

  assert.deepEqual(schema.parse(deserialize(input, schema)), model);
  assert.deepEqual(serialize(model, schema), {
    entries: [{ units: 3 }],
    units: 2,
  });
});
