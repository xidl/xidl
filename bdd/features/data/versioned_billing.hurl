# All requests use one port and one ledger; only the versioned entry differs.
GET {{base_url}}/audit
Cookie: tenant=acme
HTTP 200
[Asserts]
header "X-Billing-Version" == "2025-04-01"
jsonpath "$.attempts" == 0
jsonpath "$.commits" == 0

# Required query/header/cookie and body failures cannot enter the payment handler.
GET {{base_url}}/invoices
X-Billing-Version: 2025-01-01
Cookie: tenant=acme
HTTP 400

POST {{base_url}}/payments
Content-Type: application/json
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_1","amount":400}}
HTTP 400

POST {{base_url}}/payments
Content-Type: application/json
Idempotency-Key: hurl-1
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_1","amount":400}}
HTTP 400

POST {{base_url}}/payments
Content-Type: application/json
Idempotency-Key: hurl-1
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_1"}}
HTTP 400

GET {{base_url}}/audit
Cookie: tenant=acme
HTTP 200
[Asserts]
jsonpath "$.attempts" == 0
jsonpath "$.commits" == 0

# Explicit zero reaches pagination; absence above was a transport failure.
GET {{base_url}}/invoices?limit=0
X-Billing-Version: 2025-01-01
Cookie: tenant=acme
HTTP 200
[Asserts]
jsonpath "$" count == 0

GET {{base_url}}/invoices?limit=-1
Cookie: tenant=acme
HTTP 422
[Asserts]
jsonpath "$.type_path" == "limit"

# An unpaid invoice has no invented payment pointer in its legacy view.
GET {{base_url}}/invoices/inv_hurl
X-Billing-Version: 2025-01-01
Cookie: tenant=acme
HTTP 200
[Asserts]
jsonpath "$.amount_paid" == 0
jsonpath "$.payment_intent" not exists
jsonpath "$.payments" not exists

POST {{base_url}}/payments
X-Billing-Version: 2025-01-01
Content-Type: application/json
Idempotency-Key: hurl-1
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_1","amount":400}}
HTTP 200
[Asserts]
header "X-Billing-Version" == "2025-01-01"
jsonpath "$.amount_paid" == 400
jsonpath "$.payment_intent" == "pi_hurl_1"
jsonpath "$.payments" not exists

# A different SDK version replays the same normalized command exactly once.
POST {{base_url}}/payments
Content-Type: application/json
Idempotency-Key: hurl-1
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_1","amount":400}}
HTTP 200
[Asserts]
jsonpath "$.amount_paid" == 400
jsonpath "$.payments" count == 1
jsonpath "$.payment_intent" not exists

POST {{base_url}}/payments
Content-Type: application/json
Idempotency-Key: hurl-1
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_1","amount":401}}
HTTP 409
[Asserts]
jsonpath "$.reason" == "idempotency key reused with a different payment"

# An explicitly empty key reaches the business rule instead of becoming missing.
POST {{base_url}}/payments
Content-Type: application/json
Idempotency-Key:
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_empty","amount":1}}
HTTP 422
[Asserts]
jsonpath "$.type_path" == "key"

POST {{base_url}}/payments
Content-Type: application/json
Idempotency-Key: hurl-zero
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_zero","amount":0}}
HTTP 422
[Asserts]
jsonpath "$.type_path" == "Capture.payment.amount"

POST {{base_url}}/payments
Content-Type: application/json
Idempotency-Key: hurl-duplicate
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_1","amount":400}}
HTTP 409

GET {{base_url}}/audit
Cookie: tenant=acme
HTTP 200
[Asserts]
jsonpath "$.attempts" == 6
jsonpath "$.commits" == 1

# The next payment commits, then its legacy projection refuses to choose a pointer.
POST {{base_url}}/payments
X-Billing-Version: 2025-01-01
Content-Type: application/json
Idempotency-Key: hurl-2
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_2","amount":600}}
HTTP 422
[Asserts]
jsonpath "$.type_path" == "Invoice.payment_intent"

POST {{base_url}}/payments
X-Billing-Version: 2025-01-01
Content-Type: application/json
Idempotency-Key: hurl-2
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_2","amount":600}}
HTTP 422
[Asserts]
jsonpath "$.type_path" == "Invoice.payment_intent"

POST {{base_url}}/payments
Content-Type: application/json
Idempotency-Key: hurl-2
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_2","amount":600}}
HTTP 200
[Asserts]
jsonpath "$.amount_paid" == 1000
jsonpath "$.payments" count == 2
jsonpath "$.payments[0].amount" == 400
jsonpath "$.payments[1].amount" == 600

GET {{base_url}}/invoices/inv_hurl
X-Billing-Version: 2025-01-01
Cookie: tenant=acme
HTTP 422
[Asserts]
jsonpath "$.type_path" == "Invoice.payment_intent"

# Earlier successful replays remain byte-for-byte frozen after later payments.
POST {{base_url}}/payments
X-Billing-Version: 2025-01-01
Content-Type: application/json
Idempotency-Key: hurl-1
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_1","amount":400}}
HTTP 200
[Asserts]
jsonpath "$.amount_paid" == 400
jsonpath "$.payment_intent" == "pi_hurl_1"

POST {{base_url}}/payments
Content-Type: application/json
Idempotency-Key: hurl-1
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_1","amount":400}}
HTTP 200
[Asserts]
jsonpath "$.amount_paid" == 400
jsonpath "$.payments" count == 1

POST {{base_url}}/payments
Content-Type: application/json
Idempotency-Key: hurl-overpaid
Cookie: tenant=acme
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_excess","amount":1}}
HTTP 422

GET {{base_url}}/audit
Cookie: tenant=acme
HTTP 200
[Asserts]
jsonpath "$.attempts" == 12
jsonpath "$.commits" == 2

# Repeated cookie names follow the generated extractor's first-value rule.
# Choosing the second value would expose the other tenant's unpaid invoice.
GET {{base_url}}/invoices/inv_hurl
Cookie: tenant=acme; tenant=other
HTTP 200
[Asserts]
jsonpath "$.amount_paid" == 1000
jsonpath "$.payments" count == 2

GET {{base_url}}/invoices/inv_hurl
Cookie: tenant=intruder; tenant=acme
HTTP 403

# Repeated scalar query parameters cannot silently select another list limit.
GET {{base_url}}/invoices?limit=0&limit=100
Cookie: tenant=acme
HTTP 400

# Tenant identity scopes both invoice reads and idempotency keys.
GET {{base_url}}/invoices/inv_hurl
Cookie: tenant=other
HTTP 200
[Asserts]
jsonpath "$.amount_paid" == 0
jsonpath "$.payments" count == 0

POST {{base_url}}/payments
Content-Type: application/json
Idempotency-Key: hurl-1
Cookie: tenant=other
{"invoice_id":"inv_hurl","payment":{"payment_intent":"pi_hurl_1","amount":400}}
HTTP 200
[Asserts]
jsonpath "$.amount_paid" == 400

GET {{base_url}}/audit
Cookie: tenant=other
HTTP 200
[Asserts]
jsonpath "$.commits" == 1

GET {{base_url}}/invoices/inv_hurl
X-Billing-Version: 2025-01-01
Cookie: tenant=intruder
HTTP 403

GET {{base_url}}/invoices/missing
X-Billing-Version: 2025-01-01
Cookie: tenant=acme
HTTP 404
[Asserts]
header "X-Billing-Version" == "2025-01-01"
jsonpath "$.id" == "missing"

# A list containing an unrepresentable invoice fails as a whole.
GET {{base_url}}/invoices?limit=100
X-Billing-Version: 2025-01-01
Cookie: tenant=acme
HTTP 422
[Asserts]
jsonpath "$.type_path" == "InvoicesListResponse.r#return.Invoice.payment_intent"

GET {{base_url}}/invoices?limit=100
X-Billing-Version: 2025-08-01
Cookie: tenant=acme
HTTP 400

GET {{base_url}}/audit
Cookie: tenant=acme
HTTP 200
[Asserts]
jsonpath "$.commits" == 2
