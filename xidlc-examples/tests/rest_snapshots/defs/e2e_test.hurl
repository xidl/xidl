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

