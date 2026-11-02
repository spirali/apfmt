fn f() {
    {
        {
            SCHED_PROCESSING_READ_HISTOGRAM_STATIC.get(CMD).observe(duration_to_sec(now.saturating_duration_since(begin_instant)));
        }
    }
}
