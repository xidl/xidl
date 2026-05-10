# E2E Http Route And Body - Multiple Paths
GET {{base_url}}/v2/resources/res1?lang=en
X-Trace-Id: trace-123
HTTP 200
[Asserts]
body == "\"id:res1,lang:Some(\\\"en\\\"),trace:trace-123\""

# E2E Http Route And Body - Alternative Path
GET {{base_url}}/r/res2
X-Trace-Id: trace-456
HTTP 200
[Asserts]
body == "\"id:res2,lang:None,trace:trace-456\""

# E2E Http Route And Body - Flattened Body
POST {{base_url}}/v2/resources
Content-Type: application/json
{
    "name": "resource1",
    "tags": ["tag1", "tag2"],
    "labels": {
        "env": "prod"
    }
}
HTTP 200
[Asserts]
jsonpath "$.name" == "resource1"
jsonpath "$.tags[0]" == "tag1"
jsonpath "$.labels.env" == "prod"

# E2E Http Route And Body - Wildcard Path and Query
GET {{base_url}}/v2/files/path/to/file.txt?download=true&version=v1
HTTP 200
[Asserts]
body == "\"file:path/to/file.txt,download:true,version:Some(\\\"v1\\\")\""

# E2E Http Security - Healthz (No Security, but POST according to OpenAPI)
POST {{base_url}}/healthz
HTTP 200
[Asserts]
body == "\"ok\""

# E2E Http Security - Secure User (Basic)
GET {{base_url}}/v2/secure/users/u1
Authorization: Basic dXNlcjpwYXNz
X-Trace-Id: t1
HTTP 200
[Asserts]
body == "\"user:u1,lang:None,trace:t1\""

# E2eTypeServer - get_attribute_attr1
GET {{base_url}}/attribute/type_attr1
HTTP 200
[Asserts]
body == "\"attr1\""

# E2eTypeServer - set_attribute_attr1
POST {{base_url}}/attribute/type_attr1
Content-Type: application/json
{
    "type_attr_1": "new_attr1"
}
HTTP 204

# E2eTypeServer - simple_op
POST {{base_url}}/simple_op
{}
HTTP 204

# E2eTypeServer - simple_op_with_return1
POST {{base_url}}/simple_op_with_return1
{}
HTTP 200
[Asserts]
body == "\"simple_op_with_return1\""

# E2eTypeServer - return_with_sequence1
POST {{base_url}}/return_with_sequence1
{}
HTTP 200
[Asserts]
jsonpath "$[0]" == "s1"
jsonpath "$[1]" == "s2"

# E2eTypeServer - return_with_any
POST {{base_url}}/return_with_any
{}
HTTP 200
[Asserts]
jsonpath "$.any" == "value"

# E2eTypeServer - get_attribute_attr2
GET {{base_url}}/attribute/type_attr2
HTTP 200
[Asserts]
jsonpath "$[0]" == "attr2"

# E2eTypeServer - parameter_op3
POST {{base_url}}/parameter_op3
{
    "a": "input_a",
    "c": [1, "two"]
}
HTTP 200
[Asserts]
jsonpath "$.b" == 3
jsonpath "$.c" isCollection
jsonpath "$.c" isEmpty

