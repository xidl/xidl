# E2eAttribute - attr1
GET {{base_url}}/attribute/attr1
Accept: application/json
HTTP 200
[Asserts]
body == "\"attr1\""

# E2eAttribute - set attr1
POST {{base_url}}/attribute/attr1
Accept: application/json
Content-Type: application/json
{
    "attr_1": "new_value"
}
HTTP 204

# E2eAttribute - attr2
GET {{base_url}}/attribute/attr2
Accept: application/json
HTTP 200
[Asserts]
jsonpath "$[0]" == "attr2"

# E2eAttribute - attr4 (Enum)
GET {{base_url}}/attribute/attr4
Accept: application/json
HTTP 200
[Asserts]
body == "\"V1\""

# E2eAttribute - attr61 (Union)
GET {{base_url}}/attribute/attr61
Accept: application/json
HTTP 200
[Asserts]
jsonpath "$.tag" == "V1"
jsonpath "$.data" == 1
