// A broken pattern followed by a single-line iterable: `{` on its own line, as after `} = x` of
// `if let` (Style Guide: ` {` follows only a last line of closing delimiters)
fn f() {
    if a {
        for AllocationDescription { allocation_identifier, queue_identifier, worker_count } in items {
            x
        }
        for AllocationDescription { allocation_identifier, queue_identifier, worker_count } in load(a, b) {
            x
        }
    }
}
