# v1 keeps the removed operation
GET {{base_url}}/legacy
HTTP 200
[Asserts]
body == "legacy"

# v1 items carry no note
GET {{base_url}}/items/1
HTTP 200
[Asserts]
jsonpath "$.name" == "item"
jsonpath "$.note" not exists

# the v2 listing route does not exist yet
GET {{base_url}}/items
HTTP 404

# adopted items have no note either
POST {{base_url}}/items/1/adopt
HTTP 200
[Asserts]
jsonpath "$.name" == "item"
jsonpath "$.note" not exists

# the versioned Conflict exception has no channel before v2
POST {{base_url}}/items/9/adopt
HTTP 404
