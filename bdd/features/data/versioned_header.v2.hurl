# The server always writes its own module version; it never selects a version.

GET {{base_url}}/items/2
X-Api-Version: 2025-06
HTTP 200
[Asserts]
header "X-Api-Version" == "2025-06"
jsonpath "$.id" == "2"

GET {{base_url}}/items/3
X-Api-Version: 2025-01
HTTP 200
[Asserts]
header "X-Api-Version" == "2025-06"
