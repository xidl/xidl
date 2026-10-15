# The v2 server echoes the value it received, whatever the client pins.

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
header "X-Api-Version" == "2025-01"
