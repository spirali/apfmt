// A trailing comment that goes beyond `max_width` does not change the layout of the code in front
// of it: the code is laid out as without the comment, the comment overflows (as in rustfmt)
fn statements() {
    assert_eq!(r.multi_node_allocations[1].max_allocations, 24); // <-- Total is 25, but one is running
    assert_ne!(self.worker_id, other_worker.worker_id); // We should not receive message about ourselves
    let total_number_of_workers = first_value + second_value; // the long comment explaining this sum
    call_something(first_argument, second_argument).unwrap(); // a long comment about this call here
    let x = foo(aaaa, bbbb); /* a block comment that is long enough to overflow the line of code */
}

fn arms(mode: Mode) -> Mode {
    match mode {
        Mode::WorkerManaged => Mode::WorkerAssigned,
        Mode::Global => Mode::WorkerManaged, // To skip out of the global in case the state changes here
    }
}

struct Fields {
    first: u32,
    second_field_name: HashMap<String, Vec<u32>>, // a long comment about the field, too long to fit
}

enum Variants {
    First,
    SecondVariant(HashMap<String, Vec<u32>>), // a long comment about the variant, too long to fit
}

fn lists() {
    let v = vec![
        1, // short
        2, // a comment that is long enough to overflow the line of the code here, really long one
    ];
}

// A comment after an opening delimiter is not trailing: it moves to the next line when it does not
// fit on the line of the delimiter
fn opener() {
    if some_condition { // a comment on the opener line that fits only when it is moved to the next line
        a();
    }
}
