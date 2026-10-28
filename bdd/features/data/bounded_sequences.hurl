# The service starts with no accepted reservations.
GET {{base_url}}/accepted
HTTP 200
[Asserts]
jsonpath "$.count" == 0

# Empty is legal for bounded sequences; fixed arrays still have two entries.
POST {{base_url}}/batches
Content-Type: application/json
{"invoices": [], "windows": [], "regions": {}, "days": [[], []]}
HTTP 200
[Asserts]
jsonpath "$.invoices" count == 0
jsonpath "$.days" count == 2
jsonpath "$.days[0]" count == 0
jsonpath "$.days[1]" count == 0

# Partial batches and the inclusive maximum round-trip without padding.
POST {{base_url}}/batches
Content-Type: application/json
{"invoices": [1], "windows": [[], [1, 2]], "regions": {"eu": [1, 2]}, "days": [[1], [2]], "retry": []}
HTTP 200
[Asserts]
jsonpath "$.invoices" count == 1
jsonpath "$.invoices[0]" == 1
jsonpath "$.windows" count == 2
jsonpath "$.windows[0]" count == 0
jsonpath "$.windows[1]" count == 2
jsonpath "$.retry" count == 0

POST {{base_url}}/batches
Content-Type: application/json
{"invoices": [1, 2], "windows": [], "regions": {}, "days": [[], []]}
HTTP 200
[Asserts]
jsonpath "$.invoices" count == 2
jsonpath "$.invoices[0]" == 1
jsonpath "$.invoices[1]" == 2

# Every rejection below must happen before the reservation side effect.
POST {{base_url}}/batches
Content-Type: application/json
{"invoices": [1, 2, 3], "windows": [], "regions": {}, "days": [[], []]}
HTTP 400

POST {{base_url}}/batches
Content-Type: application/json
{"invoices": [], "windows": [[1, 2, 3]], "regions": {}, "days": [[], []]}
HTTP 400

POST {{base_url}}/batches
Content-Type: application/json
{"invoices": [], "windows": [[], [], []], "regions": {}, "days": [[], []]}
HTTP 400

POST {{base_url}}/batches
Content-Type: application/json
{"invoices": [], "windows": [], "regions": {"eu": [1, 2, 3]}, "days": [[], []]}
HTTP 400

POST {{base_url}}/batches
Content-Type: application/json
{"invoices": [], "windows": [], "regions": {}, "days": [[1, 2, 3], []]}
HTTP 400

POST {{base_url}}/batches
Content-Type: application/json
{"invoices": [], "windows": [], "regions": {}, "days": [[], []], "retry": [1, 2, 3]}
HTTP 400

# Fixed arrays are exact, not maximum-length sequences.
POST {{base_url}}/batches
Content-Type: application/json
{"invoices": [], "windows": [], "regions": {}, "days": [[]]}
HTTP 400

POST {{base_url}}/batches
Content-Type: application/json
{"invoices": [], "windows": [], "regions": {}, "days": [[], [], []]}
HTTP 400

GET {{base_url}}/accepted
HTTP 200
[Asserts]
jsonpath "$.count" == 3

GET {{base_url}}/query?values=1
HTTP 200
[Asserts]
jsonpath "$" count == 1
jsonpath "$[0]" == 1

GET {{base_url}}/query?values=1&values=2
HTTP 200
[Asserts]
jsonpath "$" count == 2
jsonpath "$[0]" == 1
jsonpath "$[1]" == 2

GET {{base_url}}/query?values=1&values=2&values=3
HTTP 400

GET {{base_url}}/headers
values: 1,2
HTTP 200
[Asserts]
jsonpath "$" count == 2
jsonpath "$[0]" == 1
jsonpath "$[1]" == 2

GET {{base_url}}/headers
values: 1,2,3
HTTP 400

GET {{base_url}}/cookies
Cookie: values=1; values=2
HTTP 200
[Asserts]
jsonpath "$" count == 2
jsonpath "$[0]" == 1
jsonpath "$[1]" == 2

GET {{base_url}}/cookies
Cookie: values=1; values=2; values=3
HTTP 400

POST {{base_url}}/upload
Content-Type: application/octet-stream
`AB`
HTTP 200
[Asserts]
jsonpath "$.count" == 2

POST {{base_url}}/upload
Content-Type: application/octet-stream
`ABC`
HTTP 400

# Invalid output is an application failure, never a successful oversized body.
GET {{base_url}}/oversized
HTTP 500

# Outbound metadata cannot publish a batch outside its declared limit.
GET {{base_url}}/exported?invalid=false
HTTP 204

GET {{base_url}}/exported?invalid=true
HTTP 500

# Negotiated JSON and binary representations enforce the same maximum.
GET {{base_url}}/download?raw=false&invalid=false
Accept: application/json
HTTP 200
[Asserts]
jsonpath "$" count == 2

GET {{base_url}}/download?raw=true&invalid=false
Accept: application/octet-stream
HTTP 200
[Asserts]
bytes count == 2

GET {{base_url}}/download?raw=false&invalid=true
Accept: application/json
HTTP 500

GET {{base_url}}/download?raw=true&invalid=true
Accept: application/octet-stream
HTTP 500

# Typed exceptions enforce bounds before sending body, headers or cookies.
GET {{base_url}}/rejected?fault=0
HTTP 409
[Asserts]
jsonpath "$.ids" count == 2
jsonpath "$.regions.eu" count == 2

GET {{base_url}}/rejected?fault=1
HTTP 500

GET {{base_url}}/rejected?fault=2
HTTP 500

GET {{base_url}}/rejected?fault=3
HTTP 500

GET {{base_url}}/rejected?fault=4
HTTP 500
