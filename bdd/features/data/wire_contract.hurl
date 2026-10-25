# Wire names are decided by the contract: explicit @rename wins over
# @rename_all, @skip members never travel, an absent optional member is
# omitted on output and accepted as null on input, and enum members use
# their canonical wire names.

POST {{base_url}}/items
{
  "item_id": "a",
  "color": "colorRed"
}
HTTP 200
[Asserts]
jsonpath "$.item_id" == "a"
jsonpath "$.color" == "colorRed"
jsonpath "$.internal_note" not exists
jsonpath "$.memo" not exists
jsonpath "$.stockGrid" not exists

# A null optional member is accepted on input and stays absent on output;
# the renamed enum value round-trips.
POST {{base_url}}/items
{
  "item_id": "b",
  "color": "blue",
  "memo": null,
  "stockGrid": null
}
HTTP 200
[Asserts]
jsonpath "$.item_id" == "b"
jsonpath "$.color" == "blue"
jsonpath "$.memo" not exists
jsonpath "$.stockGrid" not exists

# Two shelf rows each have three bin counts. A zero count is a real empty bin;
# the response preserves both axes and each bin's position.
POST {{base_url}}/items
{
  "item_id": "sku-grid",
  "color": "colorRed",
  "stockGrid": [[0, 2, 3], [4, 5, 6]]
}
HTTP 200
[Asserts]
jsonpath "$.item_id" == "sku-grid"
jsonpath "$.stockGrid" count == 2
jsonpath "$.stockGrid[0]" count == 3
jsonpath "$.stockGrid[1]" count == 3
jsonpath "$.stockGrid[0][0]" == 0
jsonpath "$.stockGrid[0][2]" == 3
jsonpath "$.stockGrid[1][0]" == 4
jsonpath "$.stockGrid[1][2]" == 6
