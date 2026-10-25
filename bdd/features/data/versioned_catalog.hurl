# Both contracts start from the same complete Head catalog.
GET {{base_url}}/catalog
X-Catalog-Version: 2026-10-01
Cookie: tenant=shop-a
HTTP 200
[Asserts]
header "X-Catalog-Version" == "2026-10-01"
jsonpath "$.products" count == 4
jsonpath "$.products[2].media" count == 3
jsonpath "$.products[2].media[2].kind" == "model_3d"
jsonpath "$.products[3].id" == "video-only"
jsonpath "$.products[3].media[0].kind" == "video"
jsonpath "$.messages" count == 0

GET {{base_url}}/catalog
X-Catalog-Version: 2026-07-01
Cookie: tenant=shop-a
HTTP 200
[Asserts]
jsonpath "$.products" count == 4
jsonpath "$.products[2].media" count == 1
jsonpath "$.products[2].media[0].url" == "https://assets.example/model/image"
jsonpath "$.products[3].media" count == 0
jsonpath "$.products[*].media[*].kind" not exists
jsonpath "$.messages" count == 0

# Filter on Head before projecting: the video-only match must remain in v1.
GET {{base_url}}/catalog?media=video
X-Catalog-Version: 2026-07-01
Cookie: tenant=shop-a
HTTP 200
[Asserts]
jsonpath "$.products" count == 3
jsonpath "$.products[0].id" == "clip"
jsonpath "$.products[1].id" == "model"
jsonpath "$.products[2].id" == "video-only"
jsonpath "$.products[2].media" count == 0
jsonpath "$.products[*].media[*].kind" not exists

# Repeated names are AND filters, retaining order-independent business meaning.
GET {{base_url}}/catalog?media=video&media=model_3d
X-Catalog-Version: 2026-07-01
Cookie: tenant=shop-a
HTTP 200
[Asserts]
jsonpath "$.products" count == 1
jsonpath "$.products[0].id" == "model"
jsonpath "$.products[0].media" count == 1

GET {{base_url}}/catalog?media=model_3d&media=video
X-Catalog-Version: 2026-10-01
Cookie: tenant=shop-a
HTTP 200
[Asserts]
jsonpath "$.products" count == 1
jsonpath "$.products[0].id" == "model"
jsonpath "$.products[0].media" count == 3

# Duplicated requirements do not duplicate products.
GET {{base_url}}/catalog?media=video&media=video
X-Catalog-Version: 2026-07-01
Cookie: tenant=shop-a
HTTP 200
[Asserts]
jsonpath "$.products" count == 3
jsonpath "$.messages" count == 0

# Unknown filters are ignored and reported, rather than changing the match set.
GET {{base_url}}/catalog?media=z_unknown&media=a_unknown&media=z_unknown
X-Catalog-Version: 2026-07-01
Cookie: tenant=shop-a
HTTP 200
[Asserts]
jsonpath "$.products" count == 4
jsonpath "$.messages" count == 2
jsonpath "$.messages[0].code" == "not_found"
jsonpath "$.messages[0].value" == "z_unknown"
jsonpath "$.messages[1].code" == "not_found"
jsonpath "$.messages[1].value" == "a_unknown"

GET {{base_url}}/catalog?media=video&media=hologram
X-Catalog-Version: 2026-10-01
Cookie: tenant=shop-a
HTTP 200
[Asserts]
jsonpath "$.products" count == 3
jsonpath "$.messages" count == 1
jsonpath "$.messages[0].value" == "hologram"

# A comma is part of one query value, not an invented second occurrence.
GET {{base_url}}/catalog?media=video%2Cmodel_3d
X-Catalog-Version: 2026-07-01
Cookie: tenant=shop-a
HTTP 200
[Asserts]
jsonpath "$.products" count == 4
jsonpath "$.messages[0].value" == "video,model_3d"

# Explicit empty text is a supplied unknown filter, distinct from zero filters.
GET {{base_url}}/catalog?media=
X-Catalog-Version: 2026-10-01
Cookie: tenant=shop-a
HTTP 200
[Asserts]
jsonpath "$.products" count == 4
jsonpath "$.messages[0].value" == ""

# The tenant is application-owned; filtering never reads a different catalog.
GET {{base_url}}/catalog?media=video
X-Catalog-Version: 2026-07-01
Cookie: tenant=shop-b
HTTP 200
[Asserts]
jsonpath "$.products" count == 1
jsonpath "$.products[0].id" == "private"
jsonpath "$.products[0].media" count == 0

GET {{base_url}}/catalog?media=video
X-Catalog-Version: 2026-07-01
Cookie: tenant=unknown
HTTP 403
[Asserts]
jsonpath "$.reason" == "unknown catalog tenant"
jsonpath "$.products" not exists

GET {{base_url}}/catalog
X-Catalog-Version: 2026-10-01
HTTP 400

# The application defaults to its concrete Head date and rejects unknown versions.
GET {{base_url}}/catalog?media=video
Cookie: tenant=shop-a
HTTP 200
[Asserts]
header "X-Catalog-Version" == "2026-10-01"
jsonpath "$.products[2].media[0].kind" == "video"

GET {{base_url}}/catalog
X-Catalog-Version: 2026-08-01
Cookie: tenant=shop-a
HTTP 400
[Asserts]
header "X-Catalog-Version" not exists
