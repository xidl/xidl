# E2eHttpScopeMatrix - scope_inherited_attr
GET {{base_url}}/attribute/scope_inherited_attr
HTTP 200
[Asserts]
body == "inherited"

# E2eHttpScopeMatrix - default_scope
POST {{base_url}}/v2/scope/default
{
    "request_body": {
        "name": "scope-test",
        "tags": [],
        "labels": {}
    }
}
HTTP 200
[Asserts]
body == "\"scope-test\""

# E2eHttpScopeMatrix - override_consumes_only
POST {{base_url}}/v2/scope/override-consumes
[FormParams]
name: Jane
age: 25
HTTP 200
[Asserts]
jsonpath "$.return" == "name:Jane,age:Some(25)"
jsonpath "$.normalized_name" == "JANE"

# E2eHttpScopeMatrix - override_produces_only
GET {{base_url}}/v2/scope/override-produces/res123
HTTP 200
# Skip body asserts for application/msgpack

# E2eHttpScopeMatrix - override_both_media
POST {{base_url}}/v2/scope/override-both
[FormParams]
name: Bob
age: 30
HTTP 200
# Skip body asserts for application/msgpack
