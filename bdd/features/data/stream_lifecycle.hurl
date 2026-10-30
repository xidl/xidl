# Before any row, the same declared Conflict is an ordinary HTTP error.
GET {{base_url}}/exports/locked
HTTP 409
[Asserts]
jsonpath "$.detail.job_id" == "locked"
jsonpath "$.detail.reason" == "locked"

# After a row the error is in-band, and no complete follows it.
GET {{base_url}}/exports/partial
HTTP 200
Content-Type: text/event-stream
[Asserts]
body contains "event: next"
body contains "\"quantity\":0"
body contains "event: error"
body contains "\"status\":409"
body contains "\"job_id\":\"partial\""
body not contains "event: complete"

GET {{base_url}}/exports/empty
HTTP 200
Content-Type: text/event-stream
[Asserts]
body contains "event: complete"
body not contains "event: next"

GET {{base_url}}/exports/retired
HTTP 410

# Conditional exports reuse the existing non-success HTTP exception channel.
GET {{base_url}}/exports/cached
HTTP 304

GET {{base_url}}/exports/partial-cached
HTTP 200
[Asserts]
body contains "event: next"
body contains "event: error"
body contains "\"status\":304"
body not contains "event: complete"

POST {{base_url}}/imports
Content-Type: application/x-ndjson
```
{"t":"heartbeat"}
{"t":"next","data":{"row":{"id":"zero","quantity":0}}}
{"t":"next","data":{"row":{"id":"雪","quantity":2}}}
{"t":"complete"}
```
HTTP 200
[Asserts]
jsonpath "$.accepted" == 2
jsonpath "$.total" == 2

# A complete frame is authoritative; following bytes never become another row.
POST {{base_url}}/imports
Content-Type: application/x-ndjson
```
{"t":"next","data":{"row":{"id":"one","quantity":1}}}
{"t":"complete"}
{"t":"next","data":{"row":{"id":"one","quantity":9}}}
```
HTTP 200
[Asserts]
jsonpath "$.accepted" == 1
jsonpath "$.total" == 1

POST {{base_url}}/imports
Content-Type: application/x-ndjson
```
{"t":"next","data":{"row":{"id":"duplicate","quantity":1}}}
{"t":"next","data":{"row":{"id":"duplicate","quantity":1}}}
{"t":"complete"}
```
HTTP 409
[Asserts]
jsonpath "$.detail.job_id" == "duplicate"

# Neither a truncated request nor a frame without data may become success.
POST {{base_url}}/imports
Content-Type: application/x-ndjson
```
{"t":"next","data":{"row":{"id":"uncommitted","quantity":0}}}
```
HTTP 400

POST {{base_url}}/imports
Content-Type: application/x-ndjson
```
{"t":"error","error":{"status":200,"body":null}}
```
HTTP 400

POST {{base_url}}/imports
Content-Type: application/x-ndjson
```
{"t":"error","error":{"status":409}}
```
HTTP 400

POST {{base_url}}/imports
Content-Type: application/x-ndjson
```
{"t":"next"}
{"t":"complete"}
```
HTTP 400
