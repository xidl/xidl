# Create user
POST {{base_url}}/
Content-Type: application/json
{
  "id": 1,
  "name": "Alice",
  "roles": ["admin"]
}
HTTP 200
[Asserts]
jsonpath "$.id" == 1
jsonpath "$.name" == "Alice"
jsonpath "$.roles[0]" == "admin"

# Get user
GET {{base_url}}/1
HTTP 200
[Asserts]
jsonpath "$.id" == 1
jsonpath "$.name" == "Alice"
jsonpath "$.roles[0]" == "admin"

# Get non-existent user
GET {{base_url}}/999
HTTP 404

# List users
GET {{base_url}}/?filter=admin
HTTP 200
[Asserts]
jsonpath "$[0].id" == 1
jsonpath "$[0].name" == "Alice"
jsonpath "$[0].roles[0]" == "admin"

# The upper half of uint32 must survive JSON and path parameters unchanged.
POST {{base_url}}/
Content-Type: application/json
{"id": 2147483648, "name": "Upper half", "roles": ["boundary"]}
HTTP 200
[Asserts]
jsonpath "$.id" == 2147483648

GET {{base_url}}/2147483648
HTTP 200
[Asserts]
jsonpath "$.id" == 2147483648
jsonpath "$.name" == "Upper half"

POST {{base_url}}/
Content-Type: application/json
{"id": 4294967295, "name": "Maximum", "roles": ["boundary"]}
HTTP 200
[Asserts]
jsonpath "$.id" == 4294967295

GET {{base_url}}/4294967295
HTTP 200
[Asserts]
jsonpath "$.id" == 4294967295
jsonpath "$.name" == "Maximum"
