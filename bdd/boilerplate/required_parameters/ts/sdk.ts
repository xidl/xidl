import assert from 'node:assert/strict';
import { RequiredParametersClient } from './required_parameters.client.js';

const base = process.argv[2];
assert(base);
const client = new RequiredParametersClient(base);
const explicit = { active: true, count: 0, label: '' };
assert.deepEqual(await client.query(0, true, ''), explicit);
assert.deepEqual(await client.header(0, true, ''), explicit);
assert.deepEqual(await client.cookie(0, true, ''), explicit);
assert.deepEqual(await client.optional(undefined, undefined, undefined), {
  active: false,
  count: 0,
  label: '',
});
console.log('PASS: explicit defaults survive required parameter SDK calls');
