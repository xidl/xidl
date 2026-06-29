use super::*;

#[tokio::test]
async fn connect_inproc_recovers_when_bound_channel_is_closed() {
    let endpoint = "inproc-closed-channel";
    let listener = InprocListener::bind(endpoint).expect("bind");
    listener.rx.lock().await.close();

    let _client = connect_inproc(endpoint).expect("connect");

    let state = REGISTRY.get(endpoint).expect("registry").clone();
    let entry = state.lock().expect("lock");
    assert!(entry.bound.is_none());
    assert_eq!(entry.pending.len(), 1);
}
