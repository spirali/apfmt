fn hugs_on_next_line() {
    {
        config.resumption = rustls::client::Resumption::store(Arc::new(NodeClientSessionStoreWrapper {
            inner: store,
            allow_resumption,
        }));
        let mut skip_batch_predicate_constants = Vec::with_capacity(if self.skip_batch_predicate.is_some() {
            1 + constant_columns.size_hint().0 * 3
        } else {
            Default::default()
        });
        let cb: Callback<RocksSnapshot> = Callback::read(Box::new(move |resp: ReadResponse<RocksSnapshot>| {
            let res = CmdRes::Snap(resp.snapshot.unwrap());
            cb2(Ok(res));
        }));
        let mut publish_paths_xxxx = paths::collect_publish_paths(paths::CollectPublishPathsOptions {
            root_dir: &root_dir,
            cli_options: &cli_options,
            diagnostics_collector: &diagnostics_collector,
        })?;
    }
}

fn hugs_on_same_line() {
    config.resumption = Resumption::store(Arc::new(Wrapper {
        inner: store,
        allow_resumption,
    }));
}

fn not_a_single_argument() {
    let output = cmd
        .args([
            "--long-argument-one",
            "--long-argument-two",
            "--long-argument-three",
        ])
        .output()?;
    let mut publish_paths_xxxxxxxx = paths::collect_publish_paths(
        root_dir,
        paths::CollectPublishPathsOptions {
            root_dir: &root_dir,
            cli_options: &cli_options,
        },
    )?;
}
