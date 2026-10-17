import assert from 'node:assert/strict';
import { test } from 'node:test';
import { z } from 'zod';
import { deserialize, serialize, xjson } from '../src/index.js';

test('record values preserve nested wire names across lazy recursive containers', () => {
  interface Node {
    label: string;
    children?: Record<string, Node>;
  }
  const node: z.ZodType<Node> = z.lazy(() =>
    z.object({
      children: z.record(z.string(), node).optional(),
      label: xjson(z.string(), { name: 'wire_label' }),
    }),
  );
  const schema = z.object({
    batches: z.array(z.record(z.string(), node.nullable())).optional(),
    flattened: xjson(z.record(z.string(), node), { flatten: true }),
    id: xjson(z.string(), { name: 'wire_id' }),
  });
  const model = {
    batches: [
      {
        empty: null,
        first: { children: { nested: { label: 'leaf' } }, label: 'root' },
      },
    ],
    flattened: { extra: { label: 'flattened' } },
    id: 'job-7',
  };
  const wire = {
    batches: [
      {
        empty: null,
        first: {
          children: { nested: { wire_label: 'leaf' } },
          wire_label: 'root',
        },
      },
    ],
    extra: { wire_label: 'flattened' },
    wire_id: 'job-7',
  };
  assert.deepEqual(serialize(model, schema), wire);
  assert.deepEqual(deserialize(wire, schema), model);
  assert.deepEqual(schema.parse(deserialize(wire, schema)), model);
  assert.deepEqual(serialize({ flattened: {}, id: 'empty' }, schema), {
    wire_id: 'empty',
  });
  assert.equal(model.batches[0].first.label, 'root');
  assert.equal(wire.batches[0].first.wire_label, 'root');
});

test('record and flattened keys remain own data properties without prototype mutation', () => {
  const value = z.object({ label: xjson(z.string(), { name: 'wire_label' }) });
  const record = z.record(z.string(), value);
  const model = JSON.parse(
    '{"__proto__":{"label":"safe"},"constructor":{"label":"also safe"}}',
  );
  const wire = JSON.parse(
    '{"__proto__":{"wire_label":"safe"},"constructor":{"wire_label":"also safe"}}',
  );
  assert.deepEqual(serialize(model, record), wire);
  assert.deepEqual(deserialize(wire, record), model);
  const flattened = z.object({ fields: xjson(record, { flatten: true }) });
  const encoded = serialize({ fields: model }, flattened);
  assert.deepEqual(encoded, wire);
  assert.equal(Object.getPrototypeOf(encoded), Object.prototype);
  assert.equal(Object.hasOwn(encoded, '__proto__'), true);
  assert.deepEqual(deserialize(wire, flattened), { fields: model });
  const inherited = Object.assign(
    Object.create({ foreign: { label: 'not owned' } }),
    { own: { label: 'owned' } },
  );
  assert.deepEqual(serialize(inherited, record), {
    own: { wire_label: 'owned' },
  });
  assert.equal(Object.hasOwn(Object.prototype, 'label'), false);
  const prototypeNamed = z.object({
    constructor: z.string().optional(),
    toString: z.string().optional(),
  });
  assert.deepEqual(deserialize({}, prototypeNamed), {});
  assert.deepEqual(serialize({}, prototypeNamed), {});
});

test('field metadata never changes a reused model definition', () => {
  const address = z.object({ zip: xjson(z.string(), { name: 'zip_code' }) });
  const normal = z.object({ address });
  const flattened = z.object({ address: xjson(address, { flatten: true }) });
  const renamed = z.object({
    address: xjson(address, { name: 'destination' }),
  });
  const model = { address: { zip: '12345' } };
  for (let run = 0; run < 2; run++) {
    assert.deepEqual(serialize(model, normal), {
      address: { zip_code: '12345' },
    });
    assert.deepEqual(serialize(model, flattened), { zip_code: '12345' });
    assert.deepEqual(serialize(model, renamed), {
      destination: { zip_code: '12345' },
    });
    assert.deepEqual(
      deserialize({ address: { zip_code: '12345' } }, normal),
      model,
    );
  }
});
