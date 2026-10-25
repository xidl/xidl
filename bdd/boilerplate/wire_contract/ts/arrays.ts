import assert from 'node:assert/strict';
import { StoreEchoRequestSchema } from './wire_contract.iface.zod.js';
import { ItemSchema } from './wire_contract.zod.js';

const stock = [
  [0, 2, 3],
  [4, 5, 6],
];
// Zod models use IDL identifiers; HTTP uses the renamed item_id and stockGrid.
const item = { color: 'colorRed', id: 'sku-grid', stock_grid: stock };
const endpoint = `${process.argv[2]}/items`;

assert.deepEqual(ItemSchema.parse(item).stock_grid, stock);
assert.deepEqual(StoreEchoRequestSchema.parse({ item }).item.stock_grid, stock);
assert.equal(
  ItemSchema.parse({ color: item.color, id: item.id }).stock_grid,
  undefined,
);
assert.equal(
  ItemSchema.parse({ ...item, stock_grid: null }).stock_grid,
  undefined,
);

for (const invalid of [
  [
    [0, 2],
    [3, 4],
    [5, 6],
  ], // transposed dimensions
  [[0, 2, 3]], // missing shelf row
  [
    [0, 2, 3],
    [4, 5],
  ], // missing bin in one row
  [[0, 2, 3], []],
  [],
  [0, 2, 3, 4, 5, 6],
  [
    [0, 2, 3],
    [4, 5, {}],
  ],
]) {
  const bad = { ...item, stock_grid: invalid };
  assert.equal(
    ItemSchema.safeParse(bad).success,
    false,
    JSON.stringify(invalid),
  );
  assert.equal(StoreEchoRequestSchema.safeParse({ item: bad }).success, false);
  const response = await fetch(endpoint, {
    body: JSON.stringify({
      color: item.color,
      item_id: item.id,
      stockGrid: invalid,
    }),
    headers: { 'Content-Type': 'application/json' },
    method: 'POST',
  });
  assert.equal(response.status, 400, JSON.stringify(invalid));
}

const response = await fetch(endpoint, {
  body: JSON.stringify({
    color: item.color,
    item_id: item.id,
    stockGrid: stock,
  }),
  headers: { 'Content-Type': 'application/json' },
  method: 'POST',
});
assert.equal(response.status, 200);
assert.deepEqual((await response.json()).stockGrid, stock);
assert.deepEqual(
  item.stock_grid,
  [
    [0, 2, 3],
    [4, 5, 6],
  ],
  'validation leaves the input intact',
);
console.log('PASS: stock grid preserves two shelves and three bins');
