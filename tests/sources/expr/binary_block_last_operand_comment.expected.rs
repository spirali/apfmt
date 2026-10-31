fn f() {
    // A comment in front of the block keeps the chain broken
    let ok = first_condition_value
        && second_condition_value // why
        && {
            let d = compute_distance(a, b);
            d < threshold_value
        };
}
