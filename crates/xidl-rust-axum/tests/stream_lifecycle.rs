#![cfg(all(feature = "client", feature = "server"))]

use axum::body::{self, Body};
use futures_util::{StreamExt, stream};
use serde::ser::Error as _;
use xidl_rust_axum::{
    Error,
    stream::{boxed_sse, decode_ndjson_body, sse_response},
};

#[derive(Clone)]
struct ExportValue(i32);
impl serde::Serialize for ExportValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.0 < 0 {
            return Err(S::Error::custom("export row cannot be encoded"));
        }
        serializer.serialize_i32(self.0)
    }
}

#[tokio::test]
async fn serialization_and_declared_failure_respect_the_first_output_boundary() {
    let before = sse_response(boxed_sse(stream::iter(vec![Err::<i32, _>(Error::new(
        409, "locked",
    ))])))
    .await;
    assert_eq!(before.status(), 409);
    let before = sse_response(boxed_sse(stream::iter(vec![Ok::<_, Error>(ExportValue(
        -1,
    ))])))
    .await;
    assert_eq!(before.status(), 500);
    for values in [
        vec![
            Ok(ExportValue(0)),
            Err(Error::new(409, "locked")),
            Ok(ExportValue(99)),
        ],
        vec![Ok(ExportValue(0)), Ok(ExportValue(-1)), Ok(ExportValue(99))],
    ] {
        let response = sse_response(boxed_sse(stream::iter(values))).await;
        assert_eq!(response.status(), 200);
        let bytes = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let wire = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(wire.matches("event: error").count(), 1, "{wire}");
        assert!(wire.contains("data: 0"));
        assert!(
            !wire.contains("event: complete")
                && !wire.contains("data: null")
                && !wire.contains("data: 99"),
            "{wire}"
        );
    }
}

#[tokio::test]
async fn request_completion_is_required_and_stops_before_appended_rows() {
    let mut input = decode_ndjson_body::<serde_json::Value>(Body::from(concat!(
        "\n{\"t\":\"heartbeat\"}\n",
        "{\"t\":\"next\",\"data\":0}\n",
        "{\"t\":\"next\",\"data\":false}\n",
        "{\"t\":\"next\",\"data\":null}\n",
        "{\"t\":\"complete\"}\n",
        "{\"t\":\"next\",\"data\":99}\n"
    )));
    let mut values = vec![];
    while let Some(value) = input.next().await {
        values.push(value.unwrap());
    }
    assert_eq!(
        values,
        vec![
            serde_json::json!(0),
            serde_json::json!(false),
            serde_json::Value::Null
        ]
    );
    for wire in [
        "",
        "{\"t\":\"next\"}\n",
        "{\"t\":\"next\",\"data\":0}\n",
        "{\"t\":\"error\",\"error\":{\"status\":200,\"body\":null}}\n",
        "{\"t\":\"error\",\"error\":{\"status\":409}}\n",
    ] {
        let mut input = decode_ndjson_body::<i32>(Body::from(wire));
        let mut failed = false;
        while let Some(value) = input.next().await {
            if let Err(error) = value {
                assert_eq!(error.http_status(), 400);
                failed = true;
            }
        }
        assert!(failed, "{wire}");
        assert!(input.next().await.is_none());
    }
}
