# get_attribute
GET {{base_url}}/attribute/host
Accept: application/json
HTTP 200
[Asserts]
body == "\"localhost\""

# set_attribute
POST {{base_url}}/attribute/host
Accept: application/json
Content-Type: application/json
{
    "host": "127.0.0.1"
}
HTTP 204

# get user info
GET {{base_url}}/user/7523
Accept: application/json
HTTP 404
[Asserts]
jsonpath "$.code" == 404
jsonpath "$.msg" == "Not Found"

# get user info legacy
GET {{base_url}}/legacy/user/7523
Accept: application/json
HTTP 404
[Asserts]
jsonpath "$.code" == 404
jsonpath "$.msg" == "Not Found"

# query user info legacy
GET {{base_url}}/v2/user?id=7523
Accept: application/json
HTTP 404
[Asserts]
jsonpath "$.code" == 404
jsonpath "$.msg" == "Not Found"

# put_key_value
POST {{base_url}}/v1/database/name/bob
Accept: application/json
HTTP 405
[Asserts]
header "Allow" == "PUT"

# delete_key_value
DELETE {{base_url}}/v1/database/name
Accept: application/json
HTTP 204

# nologin
POST {{base_url}}/login
Accept: application/json
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"login\""
jsonpath "$.code" == 401
jsonpath "$.msg" == "Unauthorized"

# nologin_realm
POST {{base_url}}/login_realm
Accept: application/json
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"request login with realm\""
jsonpath "$.code" == 401
jsonpath "$.msg" == "Unauthorized"

# login
POST {{base_url}}/login
Accept: application/json
Authorization: Basic aGVsbG86MTEx
HTTP 204
[Asserts]
header "Set-Cookie" contains "session_id=simple_session_id"

# nologin_bearer
POST {{base_url}}/login_bearer
Accept: application/json
HTTP 401
[Asserts]
jsonpath "$.code" == 401
jsonpath "$.msg" == "Unauthorized"

# bearer empty
POST {{base_url}}/login_bearer
Accept: application/json
Authorization: Bearer
HTTP 401
[Asserts]
jsonpath "$.code" == 401
jsonpath "$.msg" == "Unauthorized"

# login_bearer
POST {{base_url}}/login_bearer
Accept: application/json
Authorization: Bearer test_token
HTTP 204

# is_logined
POST {{base_url}}/is_logined
Accept: application/json
Cookie: session_id=simple_session_id
HTTP 200
[Asserts]
body == "true"
