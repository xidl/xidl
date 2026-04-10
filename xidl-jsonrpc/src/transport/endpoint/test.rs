use super::Endpoint;

#[test]
fn parse_falls_back_to_tcp_when_scheme_is_missing() {
    match Endpoint::parse("127.0.0.1:7000") {
        Endpoint::Tcp(addr) => assert_eq!(addr, "127.0.0.1:7000"),
        _ => panic!("expected tcp endpoint"),
    }
}
