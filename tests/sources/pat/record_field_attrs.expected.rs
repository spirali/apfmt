fn f() {
    let S {
        #[cfg(x)]
        a,
        b,
        ..
    } = s;
    match s {
        CloudOptions {
            // comment
            #[cfg(feature = "file_cache")]
            file_cache_ttl,
            config,
            #[cfg(feature = "cloud")]
            credential_provider: _,
        } => {}
    }
}
