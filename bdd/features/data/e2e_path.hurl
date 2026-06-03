# E2ePathSever - op_with_path
GET {{base_url}}/v1/op_with_path/val1
HTTP 200
[Asserts]
jsonpath "$[0]" == "val1"

# E2ePathSever - op_with_query
GET {{base_url}}/v1/op_with_query/val2?q=query_val
HTTP 200
[Asserts]
jsonpath "$[0]" == "val2"
jsonpath "$[1]" == "query_val"

# E2ePathSever - op_with_params
POST {{base_url}}/v1/op_with_params/path_val
{
    "a": {}
}
HTTP 200
[Asserts]
jsonpath "$[0]" == "path_val"

# E2ePathSever - op_with_query2 (Wildcard)
GET {{base_url}}/v1/op_with_query_wildcard/all/path/parts?word=hello&q=world
HTTP 200
[Asserts]
body contains "all/path/parts"
body contains "hello"
body contains "world"
