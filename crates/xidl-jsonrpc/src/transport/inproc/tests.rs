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

#[tokio::test]
async fn inproc_accept_returns_broken_pipe_after_channel_close() {
    let endpoint = "inproc-accept-closed";
    let listener = InprocListener::bind(endpoint).expect("bind");
    listener.rx.lock().await.close();

    let err = listener.accept().await.err().expect("closed channel");
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
}
