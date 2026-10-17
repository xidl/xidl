# The head version answers on its own contract, untouched.
GET {{base_url}}/items/i1
HTTP 200
{"name":"i1","owner":"ops","note":"head note","ref":"ref-i1"}

# The locked item is a native v2 exception.
GET {{base_url}}/items/locked
HTTP 409

# A v2 POST carries the actor query parameter the version introduced.
POST {{base_url}}/items?actor=cli
{
  "name": "p1",
  "owner": "ops",
  "ref": "r1"
}
HTTP 200
[Asserts]
jsonpath "$" == "draft"

# archived exists in v2, so the same head outcome projects cleanly.
POST {{base_url}}/items?actor=cli
{
  "name": "p2",
  "owner": "ops",
  "ref": "special"
}
HTTP 200
[Asserts]
jsonpath "$" == "archived"
