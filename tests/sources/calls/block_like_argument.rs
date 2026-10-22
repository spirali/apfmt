// A single block-like argument (a block, a `match` or an `if` with a simple head, a macro call with
// braces) hugs the call whenever it fits: the statement breaks elsewhere first
fn after_eq() {
    {
        {
            let one_column_with_a_string_and_one_cell_value_with_empty_string = Value::test_record(record! {
                "column" => Value::test_string(""),
            });
            let one_column_with_a_string_and_one_cell_value_with_empty_string = Value::test_record(match x {
                A => 1,
            });
            let one_column_with_a_string_and_one_cell_value_with_empty_string = Value::test_record(unsafe {
                g()
            });
            let one_column_with_a_string_and_one_cell_value_with_empty_string = Value::test_record(if x {
                g()
            } else {
                h()
            });
        }
    }
}

fn chain() {
    test().cwd(TEST_CWD).run("open sample.ini").expect_value_eq(test_record! {
        "SectionOne" => 1,
    });
    test().cwd(TEST_CWD).run("open sample.ini").expect_value_eq(match x {
        "SectionOne" => 1,
    });
}

// On one line, limited by `max_width` only, not `fn_call_width`
fn single_line() {
    Ok(unsafe { BinaryArray::new(dtype, offsets.into(), values.into(), validity) })
}

// A macro call copied on one line cannot hug
fn copied_single_line() {
    let r = Example {
        result: Some(Value::test_record(record! { "extension" => Value::test_string("rs") })),
    };
}

// A head that may break: the argument goes on its own line rather than `f(if let ...` / `{`
fn complex_head() {
    {
        ControlFlow::Continue(if let Some(height) = aexpr_projection_height(storage.get(key), None) {
            edges.outputs()[0] = height;
            SubtreeVisit::Skip
        } else {
            SubtreeVisit::Visit
        })
    }
}
