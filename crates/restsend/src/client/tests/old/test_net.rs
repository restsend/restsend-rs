use tokio::sync::mpsc::unbounded_channel;

#[test]
fn test_async_http() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let (tx, mut rx) = unbounded_channel::<String>();
    rt.spawn(async move {
        print!("hello world");
        tx.send("hello world".to_string()).unwrap();
    });
    let v: Option<String> = rx.blocking_recv();
    assert_eq!(v, Some("hello world".to_string()));
}
#[test]
fn test_blockon_with_block() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let (tx, mut rx) = unbounded_channel::<String>();
    rt.block_on(async {
        rt.spawn(async move {
            print!("hello world");
            tx.send("hello world".to_string()).unwrap();
        });
    });
    let v: Option<String> = rx.blocking_recv();
    assert_eq!(v, Some("hello world".to_string()));
}

#[test]
fn test_blockon_with_multithread() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let (tx, mut rx) = unbounded_channel::<String>();
    tokio::task::block_in_place(|| {
        rt.spawn(async move {
            print!("hello world");
            tx.send("hello world".to_string()).unwrap();
        });
    });
    let v: Option<String> = rx.blocking_recv();
    assert_eq!(v, Some("hello world".to_string()));
}
