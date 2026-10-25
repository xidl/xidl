import assert from 'node:assert/strict';
import { XidlClientError } from 'xidl-typescript-client';
import { CollectionInputsClient } from './http_collections.client.js';

const base = process.argv[2];
assert(base);
const client = new CollectionInputsClient(base);
for (const labels of [undefined, []]) {
  assert.deepEqual(await client.read([], [], [], labels), {
    names: [],
    tags: [],
    values: [],
  });
}
for (const labels of [[''], ['a&b', '雪']]) {
  assert.deepEqual(
    await client.read([0, -2, 0], ['first', 'second'], labels, labels),
    { labels, names: labels, tags: ['first', 'second'], values: [0, -2, 0] },
  );
}
for (const term of [undefined, '', 'a&b 雪']) {
  assert.deepEqual(
    await client.search(term),
    term === undefined ? {} : { term },
  );
}
for (const threshold of [undefined, 0, -2]) {
  assert.deepEqual(
    await client.numbers(0, 0, 0, threshold),
    threshold === undefined
      ? { count: 0, ratio: 0, units: 0 }
      : { count: 0, ratio: 0, threshold, units: 0 },
  );
}
assert.equal((await client.numbers(-2, 0.5, 2, undefined)).ratio, 0.5);
for (const tag of ['one,two', ' leading', 'trailing ']) {
  await assert.rejects(
    client.read([], [tag], [], undefined),
    (error: unknown) => {
      assert(error instanceof XidlClientError);
      assert.equal(error.code, 400);
      assert.match(
        error.message,
        /unrepresentable simple header collection element: tags/,
      );
      return true;
    },
  );
}
console.log('PASS: collection SDK wire occurrences');
