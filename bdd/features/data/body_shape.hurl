# A single @body parameter is the request body as-is: no wrapper object.
# The enum exercises @rename_all member naming.

POST {{base_url}}/body-item
{
  "sku": "s-1",
  "kind": "kindPlain"
}
HTTP 200
[Asserts]
jsonpath "$.sku" == "s-1"
jsonpath "$.kind" == "kindPlain"

POST {{base_url}}/body-item
{
  "sku": "s-2",
  "kind": "wrapped"
}
HTTP 200
[Asserts]
jsonpath "$.sku" == "s-2"
jsonpath "$.kind" == "wrapped"
