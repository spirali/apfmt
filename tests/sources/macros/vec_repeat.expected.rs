fn f() {
    let mut collectors = vec![SampleCollector::new(
        self.max_sample_size,
        self.max_fm_sketch_size,
        self.cm_sketch_depth,
        self.cm_sketch_width,
    ); columns_without_handle_len];
    let x = [SampleCollector::new(
        self.max_sample_size,
        self.max_fm_sketch_size,
        self.cm_sketch_depth,
        self.cm_sketch_width,
    ); columns_without_handle_len];
}
