# One IDL is the source for types, validation, wire names and OpenAPI.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 200
[Asserts]
jsonpath "$.sku_id" == "sku-雪"
jsonpath "$.internalNote" not exists
jsonpath "$.memo" not exists
jsonpath "$.stockGrid[1][2]" == 5
jsonpath "$.thumbnail[2]" == 255
jsonpath "$.revision" == 9007199254740991
jsonpath "$.mark" == "🚀"
jsonpath "$.letters[2]" == "🚀"

# Explicit optional null is accepted; skipped input never becomes an output field.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"],"memo":null,"internalNote":"must not escape"}
HTTP 200
[Asserts]
jsonpath "$.sku_id" == "sku-雪"
jsonpath "$.internalNote" not exists
jsonpath "$.memo" not exists
jsonpath "$.stockGrid[1][2]" == 5
jsonpath "$.thumbnail[2]" == 255
jsonpath "$.revision" == 9007199254740991
jsonpath "$.mark" == "🚀"
jsonpath "$.letters[2]" == "🚀"

POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"soldOut","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"],"memo":""}
HTTP 200
[Asserts]
jsonpath "$.memo" == ""
jsonpath "$.availability" == "soldOut"

POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"locked","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 409
[Asserts]
jsonpath "$.sku_id" == "locked"
jsonpath "$.reason" == "inventory locked"

# Both explicit labels and the default case preserve their discriminator.
POST {{base_url}}/changes
Content-Type: application/json
{"tag":"0","data":{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}}
HTTP 200
[Asserts]
jsonpath "$.tag" == "0"

# Both explicit labels and the default case preserve their discriminator.
POST {{base_url}}/changes
Content-Type: application/json
{"tag":"1","data":{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}}
HTTP 200
[Asserts]
jsonpath "$.tag" == "1"

# Both explicit labels and the default case preserve their discriminator.
POST {{base_url}}/changes
Content-Type: application/json
{"tag":"2","data":[{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}]}
HTTP 200
[Asserts]
jsonpath "$.tag" == "2"

# Both explicit labels and the default case preserve their discriminator.
POST {{base_url}}/changes
Content-Type: application/json
{"tag":"2","data":[]}
HTTP 200
[Asserts]
jsonpath "$.tag" == "2"

# Both explicit labels and the default case preserve their discriminator.
POST {{base_url}}/changes
Content-Type: application/json
{"tag":"default","data":"retired"}
HTTP 200
[Asserts]
jsonpath "$.tag" == "default"

# Invalid stockGrid is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid stockGrid is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[1,2,3]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid stockGrid is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[1,2,3],[4,5,6],[7,8,9]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid stockGrid is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid stockGrid is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[1,2,3,4],[5,6,7]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid stockGrid is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[1,null,3],[4,5,6]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid initial is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid initial is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"ab","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid initial is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"e\u0301","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid initial is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":null,"mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid initial is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
```raw
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"\ud800","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
```
HTTP 400

# Invalid mark is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"xy","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid mark is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
```raw
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\udfff","letters":["A","\u96ea","\ud83d\ude80"]}
```
HTTP 400

# Invalid letters is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["ab"]}
HTTP 400

# Invalid letters is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A",null]}
HTTP 400

# Invalid thumbnail is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[256],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid thumbnail is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[-1],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid thumbnail is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":"AH//","revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid revision is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":"7","initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid availability is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"in_stock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Invalid availability is rejected before business dispatch.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-\u96ea","availability":"unknown","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}
HTTP 400

# Required data is never synthesized from a missing object.
POST {{base_url}}/items
Content-Type: application/json
null
HTTP 400

# Required data is never synthesized from a missing object.
POST {{base_url}}/items
Content-Type: application/json
{}
HTTP 400

POST {{base_url}}/changes
Content-Type: application/json
{"tag":"unknown","data":{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"\ud83d\ude80","letters":["A","\u96ea","\ud83d\ude80"]}}
HTTP 400

POST {{base_url}}/changes
Content-Type: application/json
{"tag":"0"}
HTTP 400

POST {{base_url}}/changes
Content-Type: application/json
{"tag":"0","data":null}
HTTP 400

POST {{base_url}}/changes
Content-Type: application/json
{"tag":"2","data":null}
HTTP 400

POST {{base_url}}/changes
Content-Type: application/json
{"tag":"default","data":1}
HTTP 400

POST {{base_url}}/changes
Content-Type: application/json
{"tag":"2","data":[{"sku_id":"sku-\u96ea","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"ab","letters":["A","\u96ea","\ud83d\ude80"]}]}
HTTP 400

GET {{base_url}}/bindings?count=7
X-Initial: A
Cookie: session=%252F%2B%20%25
HTTP 200
X-Count: 7
[Asserts]
body == "7:A:%2F+ %"
header "Set-Cookie" contains "receipt="

GET {{base_url}}/bindings?count=7
X-Initial: ab
Cookie: session=x
HTTP 400

GET {{base_url}}/bindings?count=wrong
X-Initial: A
Cookie: session=x
HTTP 400

GET {{base_url}}/feed/ready
HTTP 200
[Asserts]
body contains "event: next"
body contains "\"tag\":\"0\""
body contains "\"tag\":\"default\""
body contains "event: complete"

GET {{base_url}}/feed/partial
HTTP 200
[Asserts]
body contains "event: next"
body contains "event: error"
body contains "\"status\":409"
body not contains "event: complete"

GET {{base_url}}/feed/locked
HTTP 409
[Asserts]
jsonpath "$.sku_id" == "locked"

# Escaped labels, mixed explicit/default branches, and strict wire decoding.
POST {{base_url}}/labels
Content-Type: application/json
{"tag":"'\"'","data":"retired"}
HTTP 200
[Asserts]
jsonpath "$.tag" == "'\"'"
POST {{base_url}}/labels
Content-Type: application/json
{"tag":"'\\\\'","data":"retired"}
HTTP 200
[Asserts]
jsonpath "$.tag" == "'\\\\'"
POST {{base_url}}/labels
Content-Type: application/json
{"tag":"'\\n'","data":"retired"}
HTTP 200
[Asserts]
jsonpath "$.tag" == "'\\n'"
POST {{base_url}}/labels
Content-Type: application/json
{"tag":"'A'","data":"retired"}
HTTP 200
[Asserts]
jsonpath "$.tag" == "'A'"
POST {{base_url}}/labels
Content-Type: application/json
{"tag":"default","data":"retired"}
HTTP 200
[Asserts]
jsonpath "$.tag" == "default"
POST {{base_url}}/labels
Content-Type: application/json
{"tag":"'Z'","data":"unknown"}
HTTP 400

POST {{base_url}}/changes
Content-Type: application/json
{"tag":"0","data":{"sku":"internal","availability":"inStock","stock_grid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":7,"initial":"A","mark":"🚀","letters":["A"]}}
HTTP 400

POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-雪","availability":"inStock","stockGrid":[[0,1.5,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740991,"initial":"A","mark":"🚀","letters":["A","雪","🚀"]}
HTTP 400

POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-雪","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9007199254740992,"initial":"A","mark":"🚀","letters":["A","雪","🚀"]}
HTTP 200
[Asserts]
jsonpath "$.revision" == 9007199254740992

# The int64 range still applies when the number is exactly representable in JavaScript.
POST {{base_url}}/items
Content-Type: application/json
{"sku_id":"sku-雪","availability":"inStock","stockGrid":[[0,1,2],[3,4,5]],"thumbnail":[0,127,255],"revision":9223372036854775808,"initial":"A","mark":"🚀","letters":["A","雪","🚀"]}
HTTP 400
