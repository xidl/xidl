# Read the oldest flat wire view; neither nested base nor newer fields leak.
GET {{base_url}}/tenants/acme/addresses/a1
Authorization: Bearer test-acme
X-Address-Version: 2026-01-01
HTTP 200
[Asserts]
header "X-Address-Version" == "2026-01-01"
jsonpath "$.id" == "a1"
jsonpath "$.deliveryReference" == "A-1"
jsonpath "$.line" == "Shared street"
jsonpath "$.countryCode" == "US"
jsonpath "$.base" not exists
jsonpath "$.previous" exists
jsonpath "$.previous[0].previous" not exists
jsonpath "$.previous[0].base" not exists
jsonpath "$.checkpoint" == "dispatch"
jsonpath "$.parent.checkpoint" == "handover"
jsonpath "$.parent.parent.checkpoint" == "packed"
jsonpath "$.parent.parent.parent" not exists
jsonpath "$.signatureRequired" not exists
jsonpath "$.deliveryNote" not exists

# Retired ancestor members and the derived zero value travel in one PATCH.
PATCH {{base_url}}/tenants/acme/addresses/a1
Authorization: Bearer test-acme
X-Address-Version: 2026-01-01
Content-Type: application/json
{"deliveryReference":"A-NEW","deliveryNote":"","line":"New street","countryCode":"CA","priority":0,"parent":{"checkpoint":"rerouted","parent":{"checkpoint":"","parent":{}}}}
HTTP 200
[Asserts]
jsonpath "$.deliveryReference" == "A-NEW"
jsonpath "$.deliveryNote" == ""
jsonpath "$.line" == "New street"
jsonpath "$.priority" == 0
jsonpath "$.parent.checkpoint" == "rerouted"
jsonpath "$.parent.parent.checkpoint" == ""
jsonpath "$.parent.parent.parent.checkpoint" not exists
jsonpath "$.parent.parent.parent.parent" not exists

GET {{base_url}}/tenants/acme/addresses/a1
Authorization: Bearer test-acme
HTTP 200
[Asserts]
header "X-Address-Version" == "2026-07-01"
jsonpath "$.street" == "New street"
jsonpath "$.countryCodeV2" == "CA"
jsonpath "$.signatureRequired" == true
jsonpath "$.deliveryReference" not exists
jsonpath "$.deliveryNote" not exists
jsonpath "$.line" not exists
jsonpath "$.parent.checkpoint" == "rerouted"

# A newer write changes current facts while retaining earlier delivery history.
PATCH {{base_url}}/tenants/acme/addresses/a1
Authorization: Bearer test-acme
X-Address-Version: 2026-07-01
Content-Type: application/json
{"street":"Head street","signatureRequired":false}
HTTP 200
[Asserts]
jsonpath "$.signatureRequired" == false

GET {{base_url}}/tenants/acme/addresses/a1
Authorization: Bearer test-acme
X-Address-Version: 2026-04-01
HTTP 200
[Asserts]
jsonpath "$.street" == "Head street"
jsonpath "$.deliveryReference" == "A-NEW"
jsonpath "$.deliveryNote" == ""
jsonpath "$.countryCodeV2" == "CA"
jsonpath "$.signatureRequired" not exists
jsonpath "$.parent.parent.checkpoint" == ""

# Capture the whole Head + history before restating the narrow view.
GET {{base_url}}/audit
HTTP 200
[Captures]
unchanged: sha256
[Asserts]
jsonpath "$[0].history[0].reference" == "A-1"
jsonpath "$[0].history[1].reference" == "A-NEW"
jsonpath "$[0].history[1].note" == ""
jsonpath "$[0].history" count == 2

PATCH {{base_url}}/tenants/acme/addresses/a1
Authorization: Bearer test-acme
X-Address-Version: 2026-01-01
Content-Type: application/json
{"deliveryReference":"A-NEW","deliveryNote":"","line":"Head street","countryCode":"CA","priority":0,"parent":{"checkpoint":"rerouted","parent":{"checkpoint":"","parent":{}}}}
HTTP 200
[Asserts]
jsonpath "$.deliveryNote" == ""
jsonpath "$.deliveryReference" == "A-NEW"

# Omitted ancestor fields preserve current values and do not append history.
PATCH {{base_url}}/tenants/acme/addresses/a1
Authorization: Bearer test-acme
X-Address-Version: 2026-04-01
Content-Type: application/json
{}
HTTP 200

GET {{base_url}}/audit
HTTP 200
[Asserts]
sha256 == {{unchanged}}

# A business conversion refusal never reaches the shared write.
PATCH {{base_url}}/tenants/acme/addresses/a1
Authorization: Bearer test-acme
X-Address-Version: 2026-01-01
Content-Type: application/json
{"countryCode":"Atlantis","deliveryReference":"must-not-persist"}
HTTP 422
[Asserts]
jsonpath "$.type_path" == "shipping::Change.countryCode"

GET {{base_url}}/audit
HTTP 200
[Asserts]
sha256 == {{unchanged}}

# Equal street/priority never causes one address's historical facts to leak.
GET {{base_url}}/tenants/acme/addresses/a2
Authorization: Bearer test-acme
X-Address-Version: 2026-01-01
HTTP 200
[Asserts]
jsonpath "$.deliveryReference" == "A-2"

GET {{base_url}}/tenants/other/addresses/a1
Authorization: Bearer test-other
X-Address-Version: 2026-01-01
HTTP 200
[Asserts]
jsonpath "$.deliveryReference" == "B-1"

GET {{base_url}}/tenants/other/addresses/a1
Authorization: Bearer test-acme
X-Address-Version: 2026-01-01
HTTP 403
[Asserts]
jsonpath "$.reason" == "tenant access denied"

PATCH {{base_url}}/tenants/other/addresses/a1
Authorization: Bearer test-acme
X-Address-Version: 2026-07-01
Content-Type: application/json
{"street":"unauthorized","signatureRequired":false,"parent":{"checkpoint":"unauthorized"}}
HTTP 403
[Asserts]
jsonpath "$.reason" == "tenant access denied"

# A present Head with no historical reference is not a 404 or fabricated string.
GET {{base_url}}/tenants/acme/addresses/orphan
Authorization: Bearer test-acme
X-Address-Version: 2026-01-01
HTTP 422
[Asserts]
jsonpath "$.type_path" == "shipping::Address.deliveryReference"

GET {{base_url}}/tenants/acme/addresses/orphan
Authorization: Bearer test-acme
X-Address-Version: 2026-07-01
HTTP 200
[Asserts]
jsonpath "$.id" == "orphan"

GET {{base_url}}/tenants/acme/addresses/missing
Authorization: Bearer test-acme
X-Address-Version: 2026-04-01
HTTP 404
[Asserts]
jsonpath "$.reason" == "address missing"

GET {{base_url}}/audit
HTTP 200
[Asserts]
sha256 == {{unchanged}}
