fn f() {
    let config = SplitConfig {
        // rustfmt leaves these lines over `max_width`
        unified_read_pool_thread_cpu_overload_threshold_ratio:
            "a very long string literal that doesn't fit",
        unified_read_pool_thread_cpu_overload_threshold_ratio:
            aaaaaaaaaaaaaaaaaaa as bbbbbbbbbbbbbbbb,
        unified_read_pool_thread_cpu_overload_threshold_ratio:
            "a very long string literal that doesn't fit at all on this line even on the next one",
        a: 1,
    };
}
