# the legacy route is removed in v2
GET {{base_url}}/legacy
HTTP 404

# v2 items carry the note
GET {{base_url}}/items/1
HTTP 200
[Asserts]
jsonpath "$.name" == "item"
jsonpath "$.note" == "v2"

# the listing route was added in v2
GET {{base_url}}/items
HTTP 200
[Asserts]
jsonpath "$[0].name" == "item"
jsonpath "$[0].note" == "v2"

# adopted items carry the note
POST {{base_url}}/items/1/adopt
HTTP 200
[Asserts]
jsonpath "$.name" == "item"
jsonpath "$.note" == "v2"

# the versioned Conflict exception raises from v2 on
POST {{base_url}}/items/2/adopt
HTTP 409
