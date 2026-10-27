# Scalar absence is different from the explicit values 0, true and "".
GET {{base_url}}/query?count=0&active=true&label=
HTTP 200
[Asserts]
jsonpath "$.count" == 0
jsonpath "$.active" == true
jsonpath "$.label" == ""

GET {{base_url}}/query?active=true&label=present
HTTP 400

GET {{base_url}}/query?count=7&label=present
HTTP 400

GET {{base_url}}/query?count=7&active=true
HTTP 400

GET {{base_url}}/query?count=invalid&active=true&label=present
HTTP 400

GET {{base_url}}/header
count: 0
active: true
label:
HTTP 200
[Asserts]
jsonpath "$.count" == 0
jsonpath "$.active" == true
jsonpath "$.label" == ""

GET {{base_url}}/header
active: true
label: present
HTTP 400

GET {{base_url}}/header
count: 7
label: present
HTTP 400

GET {{base_url}}/header
count: 7
active: true
HTTP 400

GET {{base_url}}/cookie
Cookie: count=0; active=true; label=
HTTP 200
[Asserts]
jsonpath "$.count" == 0
jsonpath "$.active" == true
jsonpath "$.label" == ""

GET {{base_url}}/cookie
Cookie: active=true; label=present
HTTP 400

GET {{base_url}}/cookie
Cookie: count=7; label=present
HTTP 400

GET {{base_url}}/cookie
Cookie: count=7; active=true
HTTP 400

# Optional omission is still accepted and owned by application defaults.
GET {{base_url}}/optional
HTTP 200
[Asserts]
jsonpath "$.note" not exists
jsonpath "$.count" == 0
jsonpath "$.active" == false
jsonpath "$.label" == ""

GET {{base_url}}/optional?count=8
active: true
Cookie: label=explicit
HTTP 200
[Asserts]
jsonpath "$.count" == 8
jsonpath "$.active" == true
jsonpath "$.label" == "explicit"
jsonpath "$.note" == "explicit"

# Optional string cookies retain presence and select the first repeated value.
GET {{base_url}}/optional
Cookie: label=
HTTP 200
[Asserts]
jsonpath "$.note" == ""

GET {{base_url}}/optional
Cookie: label=first; label=second
HTTP 200
[Asserts]
jsonpath "$.note" == "first"

# Streaming handshakes use the same presence contract as unary requests.
GET {{base_url}}/stream?count=0
active: true
Cookie: label=stream
HTTP 200
[Asserts]
body contains "\"count\":0"

GET {{base_url}}/stream
active: true
Cookie: label=stream
HTTP 400

GET {{base_url}}/stream?count=1
Cookie: label=stream
HTTP 400

GET {{base_url}}/stream?count=1
active: true
HTTP 400
