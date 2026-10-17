fn f() {
    let SplitConfig { unified_read_pool_thread_cpu_overload_threshold_ratio: Some(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa), .. } = x;
    let SplitConfig { unified_read_pool_thread_cpu_overload_threshold_ratio: Foo { aaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbb }, .. } = x;
    let SplitConfig { unified_read_pool_thread_cpu_overload_threshold_ratio: (aaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbb), .. } = x;
}
fn g() {
    let SplitConfig {
        unified_read_pool_thread_cpu_overload_threshold_ratio_xxxxxxxxxxxxxxxx: crate::module::DEFAULT_RATIO_XXXXXX,
        unified_read_pool_thread_cpu_overload_threshold_ratio: Some(aaaaaaaaaaaaaaaaaaaaaaaaaaaa),
        ..
    } = x;
}
