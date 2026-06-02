// biome-ignore-all lint/suspicious/noExplicitAny: Tests require arbitrary any types for verification.
import assert from 'node:assert';
import { test } from 'node:test';
import { z } from 'zod';
import {
  deserialize,
  deserializeZodObject,
  serialize,
  serializeZodObject,
  xjson,
} from '../src/index.js';

test('rename and ignore functionality', () => {
  const schema = z.object({
    age: xjson(z.number(), { name: 'user_age' }),
    name: xjson(z.string(), { name: 'full_name' }),
    secret: xjson(z.string(), { ignore: true }),
  });

  const original = {
    age: 30,
    name: 'Alice',
    secret: 'hidden',
  };

  // Marshal
  const serialized = serializeZodObject(original, schema);
  assert.deepStrictEqual(serialized, {
    full_name: 'Alice',
    user_age: 30,
  });

  // Unmarshal
  const deserialized = deserializeZodObject(serialized, schema);
  assert.strictEqual(deserialized.name, 'Alice');
  assert.strictEqual(deserialized.age, 30);
  assert.strictEqual(deserialized.secret, undefined);
});

test('struct flatten functionality', () => {
  const addressSchema = z.object({
    city: xjson(z.string(), { name: 'city_name' }),
    state: xjson(z.string(), { name: 'state_name' }),
  });

  const userSchema = z.object({
    address: xjson(addressSchema, { flatten: true }),
    age: xjson(z.number(), { name: 'user_age' }),
    name: xjson(z.string(), { name: 'full_name' }),
  });

  const user = {
    address: { city: 'NY', state: 'NY' },
    age: 30,
    name: 'Alice',
  };

  // Marshal
  const serialized = serializeZodObject(user, userSchema);
  assert.deepStrictEqual(serialized, {
    city_name: 'NY',
    full_name: 'Alice',
    state_name: 'NY',
    user_age: 30,
  });

  // Unmarshal
  const deserialized = deserializeZodObject(serialized, userSchema);
  assert.deepStrictEqual(deserialized, {
    address: { city: 'NY', state: 'NY' },
    age: 30,
    name: 'Alice',
  });
});

test('catch-all map/any flatten functionality', () => {
  const eventSchema = z.object({
    extra: xjson(z.any(), { flatten: true }),
    type: xjson(z.string(), { name: 'event_type' }),
  });

  const original = {
    extra: {
      event_type: 'ignored_because_named_has_priority',
      x: 10,
      y: 20,
    },
    type: 'click',
  };

  // Marshal
  const serialized = serializeZodObject(original, eventSchema);
  assert.deepStrictEqual(serialized, {
    event_type: 'click',
    x: 10,
    y: 20,
  });

  // Unmarshal
  const deserialized = deserializeZodObject(serialized, eventSchema);
  assert.deepStrictEqual(deserialized, {
    extra: {
      x: 10,
      y: 20,
    },
    type: 'click',
  });
});

test('struct flatten depth conflict resolution', () => {
  const innerASchema = z.object({
    foo: z.string(),
    id: z.string(),
  });
  const innerBSchema = z.object({
    bar: z.string(),
    id: z.string(),
  });

  const outerSchema = z.object({
    a: xjson(innerASchema, { flatten: true }),
    b: xjson(innerBSchema, { flatten: true }),
  });

  const data = {
    a: { foo: 'hello', id: 'a123' },
    b: { bar: 'world', id: 'b456' },
  };

  // Marshal: 'id' is promoted by both a and b at the same depth, so it should be dropped.
  const serialized = serializeZodObject(data, outerSchema);
  assert.deepStrictEqual(serialized, {
    bar: 'world',
    foo: 'hello',
  });
});

test('omitempty functionality', () => {
  const schema = z.object({
    arr: xjson(z.array(z.string()), { omitempty: true }),
    bool: xjson(z.boolean(), { omitempty: true }),
    num: xjson(z.number(), { omitempty: true }),
    obj: xjson(z.object({ x: z.number() }), { omitempty: true }),
    str: xjson(z.string(), { omitempty: true }),
  });

  const zeroVal = {
    arr: [],
    bool: false,
    num: 0,
    obj: {},
    str: '',
  };

  const nonZeroVal = {
    arr: ['a'],
    bool: true,
    num: 42,
    obj: { x: 1 },
    str: 'hello',
  };

  assert.deepStrictEqual(serializeZodObject(zeroVal, schema), {});
  assert.deepStrictEqual(serializeZodObject(nonZeroVal, schema), {
    arr: ['a'],
    bool: true,
    num: 42,
    obj: { x: 1 },
    str: 'hello',
  });
});

test('validation of maximum one catch-all limit', () => {
  const invalidSchema = z.object({
    extra1: xjson(z.any(), { flatten: true }),
    extra2: xjson(z.any(), { flatten: true }),
  });

  const data = {
    extra1: { a: 1 },
    extra2: { b: 2 },
  };

  assert.throws(() => {
    serializeZodObject(data, invalidSchema);
  }, /At most one catch-all flatten field/);

  assert.throws(() => {
    deserializeZodObject({ a: 1, b: 2 }, invalidSchema);
  }, /At most one catch-all flatten field/);
});
