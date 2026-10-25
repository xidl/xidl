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
