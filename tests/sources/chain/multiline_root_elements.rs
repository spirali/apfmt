// After a root broken over more lines, two or more elements are each on their own line at the
// indentation of the root, not `).first()` followed by block-indented lines
fn block_root(duration: Duration) -> String {
    if duration.num_seconds() > 0 {
        chrono::Duration::seconds(duration.num_seconds())
    } else {
        chrono::Duration::milliseconds(duration.num_milliseconds())
    }
    .to_std()
    .map(|d| humantime::format_duration(d).to_string())
    .unwrap_or_else(|_| "Invalid duration".to_string())
}

async fn call_root() {
    let client_listener = TcpListener::bind(SocketAddr::new(
        Ipv4Addr::UNSPECIFIED.into(),
        server_cfg.client_port.unwrap_or(0),
    ))
    .await
    .with_context(|| "Cannot create HQ server socket".to_string())?;
}

fn array_root() {
    let values = [
        PossibleValue::new("all").help("Detect all known resources"),
        PossibleValue::new("none").help("Do not detect any resources"),
    ]
    .into_iter()
    .chain(values);
}

fn closure_arg_root() {
    run_server_test(config, |mut handler| async move {
        handler.start_worker(config).await.unwrap();
    })
    .await
    .unwrap();
}

// A short root followed by an element broken over more lines: the elements after it are at the
// indentation of the root too
fn broken_element() {
    x.foo(
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        cccccccccccccccccccccc,
    )
    .bar()
    .baz();
}
