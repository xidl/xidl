# get_attribute
GET {{base_url}}/attribute/host
HTTP 200
[Asserts]
body == "localhost"

# set_attribute
POST {{base_url}}/attribute/host
Content-Type: text/plain
`127.0.0.1`
HTTP 204

# get user info
GET {{base_url}}/user/7523
HTTP 404
[Asserts]
jsonpath "$.code" == 404
jsonpath "$.msg" == "Not Found"

# get user info legacy
GET {{base_url}}/legacy/user/7523
HTTP 404
[Asserts]
jsonpath "$.code" == 404
jsonpath "$.msg" == "Not Found"

# query user info legacy
GET {{base_url}}/v2/user?id=7523
HTTP 404
[Asserts]
jsonpath "$.code" == 404
jsonpath "$.msg" == "Not Found"

# put_key_value
POST {{base_url}}/v1/database/name/bob
HTTP 405
[Asserts]
header "Allow" == "PUT"

# delete_key_value
DELETE {{base_url}}/v1/database/name
HTTP 204

# nologin
POST {{base_url}}/login
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"login\""
jsonpath "$.code" == 401
jsonpath "$.msg" == "Unauthorized"

# nologin_realm
POST {{base_url}}/login_realm
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"request login with realm\""
jsonpath "$.code" == 401
jsonpath "$.msg" == "Unauthorized"

# login
POST {{base_url}}/login
Authorization: Basic aGVsbG86MTEx
HTTP 204
[Asserts]
header "Set-Cookie" contains "session_id=simple_session_id"

# nologin_bearer
POST {{base_url}}/login_bearer
HTTP 401
[Asserts]
jsonpath "$.code" == 401
jsonpath "$.msg" == "Unauthorized"

# bearer empty
POST {{base_url}}/login_bearer
Authorization: Bearer
HTTP 401
[Asserts]
jsonpath "$.code" == 401
jsonpath "$.msg" == "Unauthorized"

# login_bearer
POST {{base_url}}/login_bearer
Authorization: Bearer test_token
HTTP 204

# is_logined
POST {{base_url}}/is_logined
Cookie: session_id=simple_session_id
HTTP 200
[Asserts]
body == "true"
