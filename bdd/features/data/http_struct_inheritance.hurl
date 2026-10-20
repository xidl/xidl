GET {{base_url}}/participant
HTTP 200
[Asserts]
jsonpath "$.id" == "u-1"
jsonpath "$.role" == "admin"
jsonpath "$.base" not exists

POST {{base_url}}/echo
Content-Type: application/json
{"id":"u-2","role":"member"}
HTTP 200
[Asserts]
jsonpath "$.id" == "u-2"
jsonpath "$.role" == "member"
jsonpath "$.base" not exists
