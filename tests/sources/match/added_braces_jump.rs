// `return` in braces added around a match arm or closure body gets `;`, as in a block of the source
fn f() {
    let glob_pattern = match glob_pattern_input {
        Value::String { val, .. } => val,
        _ => return Err(ShellError::IncorrectValue {
            msg: "Incorrect glob pattern supplied to glob. Please use string or glob only.".to_string(),
            val_span: call.head,
        }),
    };
    let g = || return Err(ShellError::IncorrectValue { msg: "Incorrect glob pattern supplied to glob.".to_string(), val_span: call.head });
}
