# Test download bytes
GET {{base_url}}/download
HTTP 200
[Asserts]
header "Content-Type" == "application/octet-stream"
body == "hello world"

# Test upload bytes
POST {{base_url}}/upload
Content-Type: application/octet-stream
```
this is a test stream chunk
```
HTTP 200
[Asserts]
body contains "this is a test stream chunk"
