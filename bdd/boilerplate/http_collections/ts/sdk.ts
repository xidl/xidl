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
