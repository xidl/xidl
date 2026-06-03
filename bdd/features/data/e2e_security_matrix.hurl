# E2eHttpSecurityMatrix - inherited_security
GET {{base_url}}/v2/security/inherited/res1?org_key=secret
Authorization: Basic dXNlcjpwYXNz
X-Trace-Id: trace-sec
HTTP 200
[Asserts]
body == "\"res1:trace-sec\""

# E2eHttpSecurityMatrix - public_ping
GET {{base_url}}/v2/security/public/ping
HTTP 200
[Asserts]
body == "\"pong\""
