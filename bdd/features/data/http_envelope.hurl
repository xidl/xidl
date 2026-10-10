GET {{base_url}}/secondary
Accept: text/plain
HTTP 200
Content-Type: text/plain
[Asserts]
body == "second-idl"

GET {{base_url}}/file?key=meta
Accept: application/json
HTTP 200
Content-Type: application/json
ETag: meta-v1
[Asserts]
jsonpath "$.wire_id" == "meta"
jsonpath "$.label" == "detail"
jsonpath "$.id" not exists
jsonpath "$.details" not exists
jsonpath "$.note" not exists
header "X-Tag" contains "001"
header "X-Tag" contains "\"quoted\""
cookie "cached" == "false"

GET {{base_url}}/file?key=raw
Accept: application/octet-stream
HTTP 200
Content-Type: application/octet-stream
ETag: meta-v1
[Asserts]
body == "envelope-bytes"
header "X-Tag" contains "001"
header "X-Tag" contains "\"quoted\""
cookie "cached" == "false"

GET {{base_url}}/file?key=text
Accept: text/plain
HTTP 200
Content-Type: text/plain
ETag: meta-v1
[Asserts]
body == "envelope-text"
header "X-Tag" contains "001"
header "X-Tag" contains "\"quoted\""
cookie "cached" == "false"

# A supported Accept still rejects a different returned representation.
GET {{base_url}}/file?key=raw
Accept: application/json
HTTP 406

GET {{base_url}}/file?key=meta
Accept: application/octet-stream
HTTP 406

# Declared errors must remain reachable regardless of success representation.
GET {{base_url}}/file?key=missing
Accept: application/octet-stream
HTTP 404
[Asserts]
jsonpath "$.msg" == "no such file"

GET {{base_url}}/fresh
HTTP 304
ETag: same-tag
[Asserts]
body == ""

DELETE {{base_url}}/file
If-Match: stale
HTTP 412
ETag: current-tag
[Asserts]
jsonpath "$.msg" == "stale revision"
