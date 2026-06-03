# E2eHttpDefaultsMatrix - delete_resource_default_query
DELETE {{base_url}}/v2/defaults/resources?id=r1&revision=10
HTTP 200
[Asserts]
body == "\"r1:10\""

# E2eHttpDefaultsMatrix - replace_resource_default_body
PUT {{base_url}}/v2/defaults/resources/r2
{
    "name": "new_name",
    "alias": "better_name"
}
HTTP 200
[Asserts]
jsonpath "$.name" == "new_name"
jsonpath "$.alias" == "better_name"
jsonpath "$.tags[0]" == "r2"
