fn f() {
    {
        let buf_writer = |thread_no,
                          write_buffer: &mut Vec<_>,
                          csv_serializer: &mut CsvSerializer| {
            // and allows writing `pl.concat([df] * 100, rechunk=False).write_csv()` as the rechunk aa
            let a = 1;
        };
    }
}
