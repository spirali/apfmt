async fn f() {
    while history.recv_many(&mut events, capacity).await != 0 {
        if tx
            .send_all(&mut futures::stream::iter(events).map(|e| Ok(ToClientMessage::Event(e))))
            .await
            .is_err()
        {
            break;
        }
        process(&*futures::stream::iter(events).map(|e| Ok(ToClientMessage::Event(e))).count());
        process(futures::stream::iter(events).map(|e| Ok(ToClientMessage::Event(e))).count() as u64);
    }
}
