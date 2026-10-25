# Zero occurrences encode the required logical empty list; optional lists stay absent.
GET {{base_url}}/collections
HTTP 200
[Asserts]
jsonpath "$.values" count == 0
jsonpath "$.tags" count == 0
jsonpath "$.names" count == 0
jsonpath "$.labels" not exists

# Singleton and repeated values retain order, duplicates and explicit empty elements.
GET {{base_url}}/collections?values=0&labels=
tags:
Cookie: names=
HTTP 200
[Asserts]
jsonpath "$.values" count == 1
jsonpath "$.values[0]" == 0
jsonpath "$.tags" count == 1
jsonpath "$.tags[0]" == ""
jsonpath "$.names" count == 1
jsonpath "$.names[0]" == ""
jsonpath "$.labels" count == 1
jsonpath "$.labels[0]" == ""

GET {{base_url}}/collections?values=0&values=-2&values=0&labels=first&labels=second
tags: first, second
Cookie: names=first; names=second
HTTP 200
[Asserts]
jsonpath "$.values" count == 3
jsonpath "$.values[0]" == 0
jsonpath "$.values[1]" == -2
jsonpath "$.values[2]" == 0
jsonpath "$.tags" count == 2
jsonpath "$.tags[0]" == "first"
jsonpath "$.tags[1]" == "second"
jsonpath "$.names" count == 2
jsonpath "$.names[0]" == "first"
jsonpath "$.names[1]" == "second"
jsonpath "$.labels" count == 2
jsonpath "$.labels[0]" == "first"
jsonpath "$.labels[1]" == "second"

GET {{base_url}}/collections?values=1&values=invalid
HTTP 400

# Empty numeric elements stay invalid even beside an explicit zero.
GET {{base_url}}/collections?values=&values=2
HTTP 400

GET {{base_url}}/collections?values=0&values=%20
HTTP 400

GET {{base_url}}/collections?values=0%0A
HTTP 400

GET {{base_url}}/collections?values=0x10
HTTP 400

GET {{base_url}}/collections?values=1e2
HTTP 400

GET {{base_url}}/collections?values=1.0
HTTP 400

# Scalar aliases and numeric header/cookie inputs use the same decimal grammar.
GET {{base_url}}/numbers?count=0
ratio: 0
Cookie: units=0
HTTP 200
[Asserts]
jsonpath "$.count" == 0
jsonpath "$.ratio" == 0
jsonpath "$.units" == 0
jsonpath "$.threshold" not exists

GET {{base_url}}/numbers?count=%2B2&threshold=0
ratio: -.5e2
Cookie: units=%2B2
HTTP 200
[Asserts]
jsonpath "$.count" == 2
jsonpath "$.ratio" == -50
jsonpath "$.units" == 2
jsonpath "$.threshold" == 0

GET {{base_url}}/numbers?count=
ratio: 0
Cookie: units=0
HTTP 400

GET {{base_url}}/numbers?count=%20
ratio: 0
Cookie: units=0
HTTP 400

GET {{base_url}}/numbers?count=0&threshold=
ratio: 0
Cookie: units=0
HTTP 400

GET {{base_url}}/numbers?count=0&threshold=%09
ratio: 0
Cookie: units=0
HTTP 400

GET {{base_url}}/numbers?count=0
ratio:
Cookie: units=0
HTTP 400

GET {{base_url}}/numbers?count=0
ratio: 0x10
Cookie: units=0
HTTP 400

GET {{base_url}}/numbers?count=0
ratio: 0
Cookie: units=
HTTP 400

GET {{base_url}}/numbers?count=0
ratio: 0
Cookie: units=-1
HTTP 400

GET {{base_url}}/numbers?count=0
ratio: 0
Cookie: units=-0
HTTP 400

# Non-finite float text stays invalid on both servers: JSON cannot carry it
# and a committed inf would serialize as null downstream.
GET {{base_url}}/numbers?count=0
ratio: inf
Cookie: units=0
HTTP 400

GET {{base_url}}/numbers?count=0
ratio: NaN
Cookie: units=0
HTTP 400

GET {{base_url}}/numbers?count=0
ratio: 1e400
Cookie: units=0
HTTP 400

# A scalar query parameter rejects repeated occurrences instead of taking
# the first value.
GET {{base_url}}/numbers?count=0&count=1
ratio: 0
Cookie: units=0
HTTP 400

# Malformed cookie pairs fail extraction; invalid numeric values fail parsing.
GET {{base_url}}/numbers?count=0
ratio: 0
Cookie: garbage
HTTP 400

GET {{base_url}}/numbers?count=0
ratio: 0
Cookie: units=100%
HTTP 400

# Missing optional text is distinct from a supplied empty string.
GET {{base_url}}/search
HTTP 200
[Asserts]
jsonpath "$.term" not exists

GET {{base_url}}/search?term=
HTTP 200
[Asserts]
jsonpath "$.term" == ""

# Streaming handshakes use the same repeated-value decoder as unary extraction.
GET {{base_url}}/stream
HTTP 200
[Asserts]
body not contains "\"values\""

GET {{base_url}}/stream?values=0
HTTP 200
[Asserts]
body contains "\"values\":[0]"

GET {{base_url}}/stream?values=0&values=2
HTTP 200
[Asserts]
body contains "\"values\":[0,2]"

GET {{base_url}}/stream?values=invalid
HTTP 400

GET {{base_url}}/stream?values=
HTTP 400
