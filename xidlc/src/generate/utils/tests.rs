use super::*;

#[test]
fn test_parse_timestamp() {
    let timestamp = [
        //
        0,
        1000,
        i64::MAX,
        i64::MIN,
    ];

    for case in timestamp {
        let _ = format_timestamp_filter(case);
    }
}
