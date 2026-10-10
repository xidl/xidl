# All responses come from one operation with a raises list.
GET {{base_url}}/files/ok
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$.id" == "ok"
jsonpath "$.etag" == "v2"

GET {{base_url}}/files/cached
HTTP 304
ETag: "v2"
[Asserts]
body == ""
header "Content-Type" not exists

GET {{base_url}}/files/stale
HTTP 412
Content-Type: application/json
ETag: "v2"
[Asserts]
cookie "session" == "renewed"
cookie "retry" exists
header "X-Hint" contains "reload"
header "X-Hint" contains "retry"
header "Set-Cookie" contains "attempt=first"
header "Set-Cookie" contains "attempt=second"
jsonpath "$.code" == 41201
jsonpath "$.message" == "stale revision"
jsonpath "$.detail" == "reload before retry"
jsonpath "$.msg" not exists
jsonpath "$.etag" not exists
jsonpath "$.session_id" not exists
jsonpath "$.retry_token" not exists
jsonpath "$.attempts" not exists
jsonpath "$.hints" not exists

GET {{base_url}}/files/stale-minimal
HTTP 412
Content-Type: application/json
ETag: "v2"
[Asserts]
cookie "session" == "renewed"
cookie "retry" not exists
cookie "attempt" not exists
header "X-Hint" not exists
jsonpath "$.code" == 41201
jsonpath "$.message" == "stale revision"

GET {{base_url}}/files/missing
HTTP 404
Content-Type: application/json
[Asserts]
jsonpath "$.code" == 40401
jsonpath "$.msg" == "not found"
header "ETag" not exists
header "Set-Cookie" not exists

# Undeclared failures still use the framework channel.
GET {{base_url}}/files/framework
HTTP 500
[Asserts]
jsonpath "$.code" == 500
jsonpath "$.msg" == "storage unavailable"
