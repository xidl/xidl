# E2eHttpForm - submit_profile
POST {{base_url}}/v2/forms/profile
[FormParams]
name: John
age: 30
HTTP 200
[Asserts]
jsonpath "$.return" == "name:John,age:Some(30)"
jsonpath "$.normalized_name" == "JOHN"
