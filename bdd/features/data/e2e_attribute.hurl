# E2eAttribute - attr1
GET {{base_url}}/attribute/attr1
HTTP 200
[Asserts]
body == "attr1"

# E2eAttribute - set attr1
POST {{base_url}}/attribute/attr1
Content-Type: text/plain
`new_value`
HTTP 204

# E2eAttribute - attr2
GET {{base_url}}/attribute/attr2
HTTP 200
[Asserts]
jsonpath "$[0]" == "attr2"

# E2eAttribute - attr4 (Enum)
GET {{base_url}}/attribute/attr4
HTTP 200
[Asserts]
body == "\"V1\""

# E2eAttribute - attr61 (Union)
GET {{base_url}}/attribute/attr61
HTTP 200
[Asserts]
jsonpath "$.tag" == "V1"
jsonpath "$.data" == 1
