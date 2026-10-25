# Typed bodies echo back exactly what the caller sent.
POST {{base_url}}/probe
Content-Type: application/json
{"id": "p1", "locked": false, "minutes": 7}
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$.id" == "p1"
jsonpath "$.locked" == false
jsonpath "$.minutes" == 7

# A wrong-typed JSON value is refused, never coerced: "false" must not
# become true, in either generated language.
POST {{base_url}}/probe
Content-Type: application/json
{"id": "p1", "locked": "false", "minutes": 1}
HTTP 400

# Null never becomes a string.
POST {{base_url}}/probe
Content-Type: application/json
{"id": null, "locked": true, "minutes": 1}
HTTP 400

# Null never becomes a number.
POST {{base_url}}/probe
Content-Type: application/json
{"id": "p1", "locked": true, "minutes": null}
HTTP 400

# An empty string never becomes a number.
POST {{base_url}}/probe
Content-Type: application/json
{"id": "p1", "locked": true, "minutes": ""}
HTTP 400

# Scalar transports carry text: the query position coerces "true" and "5"
# into the declared boolean and long.
GET {{base_url}}/probe?deep=true&count=5
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$.id" == "query"
jsonpath "$.locked" == true
jsonpath "$.minutes" == 5

# Coercion is not invention: text that is not the declared type is refused.
GET {{base_url}}/probe?deep=banana&count=5
HTTP 400

GET {{base_url}}/probe?deep=true&count=five
HTTP 400
