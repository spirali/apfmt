fn f() {
    fail_point!("before_clean_overlap_ranges", |_| Ok(()));
    let server = server::http(move |req| async move {
        // not iamatoken
        assert_eq!(1, 2);
        http::Response::default()
    });
    let x = y.and_then(|idx| {
        // SAFETY: x
        unsafe { self.get_unchecked(idx) }
    });
}
