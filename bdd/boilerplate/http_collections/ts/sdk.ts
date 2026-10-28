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
const numbers = await client.numbers(2, -50, 2, 0);
assert.equal(numbers.count, 2);
assert.equal(numbers.ratio, -50);
assert.equal(numbers.units, 2);
assert.equal(numbers.threshold, 0);
const absent = await client.numbers(0, 0, 0, undefined);
assert.equal(absent.threshold, undefined);
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
for (const letters of [[], ['a'], ['a', 'b', 'a']]) {
  assert.deepEqual(await client.letters(letters), letters);
  assert.deepEqual(await client.letter_aliases(letters), letters);
  assert.deepEqual(await client.wide_letters(letters), letters);
}
assert.equal(await client.initial('A'), 'A');
const cookieLetters = [',', ' ', '雪'];
assert.deepEqual(await client.cookie_letters(cookieLetters), cookieLetters);
for (const letter of [',', ' ']) {
  await assert.rejects(client.letters([letter]), (error: unknown) => {
    assert(error instanceof XidlClientError);
    assert.equal(error.code, 400);
    assert.match(
      error.message,
      /unrepresentable simple header collection element: letters/,
    );
    return true;
  });
}
console.log('PASS: collection SDK wire occurrences');
