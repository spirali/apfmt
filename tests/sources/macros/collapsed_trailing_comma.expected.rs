fn f() {
    assert_eq!(x, Foo(a));
    assert_eq!(req_ver, RequestedVersion::Latest(ReleaseChannel::Canary));
    m!(vec![a]);
    assert!((a, b) == c);
}
