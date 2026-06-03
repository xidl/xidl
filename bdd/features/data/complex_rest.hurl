# Create user
POST {{base_url}}/
Content-Type: application/json
{
  "user": {
    "id": 1,
    "name": "Alice",
    "roles": ["admin"]
  }
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
