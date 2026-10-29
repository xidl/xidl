# GET stays callable beside the operation published as a webhook.
GET {{base_url}}/events
HTTP 200
[Asserts]
jsonpath "$" == 0

# A custom event name is an ordinary string, with a real business response.
POST {{base_url}}/events
Content-Type: application/json
X-Delivery-ID: first
{"event_type":"business.custom","invoice_id":"inv-1","amount":12}
HTTP 200
[Asserts]
jsonpath "$.event_type" == "business.custom"
jsonpath "$.invoice_id" == "inv-1"
jsonpath "$.accepted" == true

# Replaying the same delivery does not apply the business operation twice.
POST {{base_url}}/events
Content-Type: application/json
X-Delivery-ID: first
{"event_type":"business.custom","invoice_id":"inv-1","amount":12}
HTTP 200
[Asserts]
jsonpath "$.invoice_id" == "inv-1"

GET {{base_url}}/events
HTTP 200
[Asserts]
jsonpath "$" == 1

# Reusing identity for another payload is a declared business error.
POST {{base_url}}/events
Content-Type: application/json
X-Delivery-ID: first
{"event_type":"invoice.paid","invoice_id":"inv-2","amount":12}
HTTP 409
[Asserts]
jsonpath "$.reason" == "delivery payload conflict"

# A rejected conflict must leave the original delivery intact.
POST {{base_url}}/events
Content-Type: application/json
X-Delivery-ID: first
{"event_type":"business.custom","invoice_id":"inv-1","amount":12}
HTTP 200
[Asserts]
jsonpath "$.invoice_id" == "inv-1"

# The application owns amount rules, without compiler-specific event dispatch.
POST {{base_url}}/events
Content-Type: application/json
X-Delivery-ID: invalid
{"event_type":"invoice.paid","invoice_id":"inv-2","amount":-1}
HTTP 409
[Asserts]
jsonpath "$.reason" == "invalid invoice"

# Invalid wire input is rejected before application state changes.
POST {{base_url}}/events
Content-Type: application/json
X-Delivery-ID: malformed
{"event_type":"invoice.paid","invoice_id":"inv-2","amount":"12"}
HTTP 400

POST {{base_url}}/events
Content-Type: application/json
X-Delivery-ID: malformed
{"invoice_id":"inv-2","amount":12}
HTTP 400

POST {{base_url}}/events
Content-Type: application/json
X-Delivery-ID: malformed
`{"event_type":"invoice.paid"`
HTTP 400

GET {{base_url}}/events
HTTP 200
[Asserts]
jsonpath "$" == 1

# A new identity accepts an optional note and a different string event value.
POST {{base_url}}/events
Content-Type: application/json
X-Delivery-ID: second
{"event_type":"invoice.paid","invoice_id":"inv-2","amount":0,"note":"settled"}
HTTP 200
[Asserts]
jsonpath "$.event_type" == "invoice.paid"
jsonpath "$.accepted" == true

GET {{base_url}}/events
HTTP 200
[Asserts]
jsonpath "$" == 2
