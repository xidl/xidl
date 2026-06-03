# Media Types - Form URL Encoded
POST {{base_url}}/v1/forms/profile
[FormParams]
name: John
age: 25
HTTP 200
[Asserts]
jsonpath "$.return" == "John:25"
jsonpath "$.normalized_name" == "JOHN"

# Media Types - Msgpack
GET {{base_url}}/v1/msgpack/users/u123
HTTP 200
# Note: hurl might not support msgpack decoding easily in asserts if it's binary,
# but we can at least check the status code.
