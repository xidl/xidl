# The version header travels with the request and the server echoes the
# received value back; a missing header is accepted because servers never
# resolve versions - applications do.

GET {{base_url}}/items/1
X-Api-Version: 2025-01
HTTP 200
[Asserts]
header "X-Api-Version" == "2025-01"
jsonpath "$.id" == "1"

GET {{base_url}}/items/2
HTTP 200
[Asserts]
jsonpath "$.id" == "2"
