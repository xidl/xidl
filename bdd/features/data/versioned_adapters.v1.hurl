# A v1 GET projects the head item onto the exact v1 wire shape: the retired
# reference rides again, head-only members are gone.
GET {{base_url}}/items/i1
HTTP 200
{"name":"i1","ref":"ref-i1"}

# The head-only Locked exception reaches v1 as its NotFound.
GET {{base_url}}/items/locked
HTTP 404

# A v1 POST needs no actor: the upcast hook supplies what the head requires.
POST {{base_url}}/items
{
  "item": {
    "name": "p1",
    "ref": "r1"
  }
}
HTTP 200
[Asserts]
jsonpath "$" == "draft"

# A head outcome the older version cannot represent must be refused on the
# wire, never invented into an older value.
POST {{base_url}}/items
{
  "item": {
    "name": "p2",
    "ref": "special"
  }
}
HTTP 500
