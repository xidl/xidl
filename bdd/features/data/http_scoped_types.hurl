POST {{base_url}}/scoped/exchange
Content-Type: application/json
{"items":[{"name":"Alice"}],"root":{"id":42}}
HTTP 200
[Asserts]
jsonpath "$.label" == "Alice:42"

POST {{base_url}}/scoped/local
Content-Type: application/json
{"value":7}
HTTP 200
[Asserts]
jsonpath "$.value" == 7

POST {{base_url}}/scoped/root
Content-Type: application/json
{"id":99}
HTTP 200
[Asserts]
jsonpath "$.id" == 99

GET {{base_url}}/scoped/failure
HTTP 429
[Asserts]
header "X-Hint" == "\"Busy\""
