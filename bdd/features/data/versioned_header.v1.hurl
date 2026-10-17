# The server always writes its own module version; it never selects a version.

GET {{base_url}}/items/1
X-Api-Version: 2025-01
HTTP 200
[Asserts]
header "X-Api-Version" == "2025-01"
jsonpath "$.id" == "1"

GET {{base_url}}/items/2
HTTP 200
[Asserts]
header "X-Api-Version" == "2025-01"
jsonpath "$.id" == "2"
