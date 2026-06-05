# Smart City - Get Stop ETA
GET {{base_url}}/v1/stops/S100?line=2&lang=en
Authorization: Bearer mytoken
HTTP 200
[Asserts]
jsonpath "$.return" == "S100"
jsonpath "$.eta_seconds" == 240
jsonpath "$.destination" == "Central Station"

# Smart City - List Nearby Stops
GET {{base_url}}/v1/stops/S100/nearby
Authorization: Bearer mytoken
HTTP 200
[Asserts]
jsonpath "$[0]" == "S100-A"
jsonpath "$[1]" == "S100-B"

# Smart City - Download Asset
GET {{base_url}}/v1/assets/docs/readme.txt?version=v1
Authorization: Bearer mytoken
HTTP 200
[Asserts]
jsonpath "$.content_type" == "text/plain"
jsonpath "$.etag" == "etag-demo"
jsonpath "$.return[0]" == 97 # 'a'

# Smart City - Reserve Lot
POST {{base_url}}/v1/parking/lots/P1/reserve
X-Reserve-Key: reserve-key
{
    "plate_number": "A-12345",
    "minutes": 30
}
HTTP 200
[Asserts]
jsonpath "$.return" == "resv-P1"
jsonpath "$.reservation_state" == "CONFIRMED"

# Smart City - Get Profile
GET {{base_url}}/v1/citizens/C100
Authorization: Bearer mytoken
HTTP 200
[Asserts]
jsonpath "$.return" == "C100"
jsonpath "$.display_name" == "Taylor"

# Smart City - Get Device Status
GET {{base_url}}/v1/devices/D100?locale=en-US
X-Trace-Id: trace-001
Cookie: session=sess-abc
Authorization: Bearer mytoken
HTTP 200
[Asserts]
jsonpath "$.return" == "device:D100"
jsonpath "$.trace_echo" == "trace-001"
jsonpath "$.session_echo" == "sess-abc"

# Smart City - Get API Version
GET {{base_url}}/attribute/api_version
Authorization: Bearer mytoken
HTTP 200
[Asserts]
body == "v2.0.0"
