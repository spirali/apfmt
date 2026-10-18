// A broken pattern followed by a broken iterable: ` {` after its closing delimiters, as in `if let`;
// rustfmt breaks after `in` and puts the iterable on its own line
fn f() {
    if a {
        for AllocationDescription {
            allocation_identifier,
            queue_identifier,
            worker_count,
        } in load_all_the_allocation_descriptions(
            first_argument_value,
            second_argument_value,
            third,
        ) {
            x
        }
    }
}
