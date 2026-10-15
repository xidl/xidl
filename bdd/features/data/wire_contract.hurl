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

# A null optional member is accepted on input and stays absent on output;
# the renamed enum value round-trips.
POST {{base_url}}/items
{
  "item_id": "b",
  "color": "blue",
  "memo": null
}
HTTP 200
[Asserts]
jsonpath "$.item_id" == "b"
jsonpath "$.color" == "blue"
jsonpath "$.memo" not exists
