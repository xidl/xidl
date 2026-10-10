GET {{base_url}}/document
HTTP 200
[Asserts]
header "ETag" == "document-v1"
jsonpath "$.title" == "Guide"
jsonpath "$.return" not exists

GET {{base_url}}/scalar
HTTP 200
[Asserts]
header "ETag" == "scalar-v1"
body == "Guide"

GET {{base_url}}/output
HTTP 200
[Asserts]
header "ETag" == "output-v1"
jsonpath "$.title" == "Guide"
jsonpath "$.document" not exists

POST {{base_url}}/upload
Content-Type: text/markdown
```
# Guide
```
HTTP 200
[Asserts]
jsonpath "$.media_type" == "text/markdown"
jsonpath "$.size" == 8

POST {{base_url}}/rename
Content-Type: application/json
{"title":"Revised"}
HTTP 200
ETag: renamed-v1
[Asserts]
jsonpath "$.title" == "Revised"
