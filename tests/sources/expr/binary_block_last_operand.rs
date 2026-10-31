fn f() {
    // Only a plain block, after operands on one line
    let should_merge = is_overlapping || { let d = compute_distance(a, b); d < threshold_value };
    let ok = self.len() == other.len() && self.null_count() == other.null_count() && { let eq = self.equal_missing(other); eq.all() };
    let ok = self.len_of_something_long() == other.len_of_something_long() && self.null_count_of_something() == other.null_count_of_something() && { let eq = self.equal_missing(other); eq.all() };
    let v = value_of_the_first_operand + match kind { Kind::A => first_value_long, Kind::B => second_value_long };
    let v = condition_value_long || if other_condition { compute_first_value() } else { compute_second() };
    let v = condition_value_long || unsafe { let p = ptr.read(); p.is_valid_pointer_value() };
    let v = condition_value_long || loop { if done_with_the_loop() { break true; } };
    let v = condition_value_long || compute_something(first_argument_value, second_argument_value, third);
    let v = first_condition || second_condition && { let d = compute_distance(a, b); d < threshold_value };
    if input.peek(token::Group) && { let group = parse_group(&input.fork())?; group.content.is_empty() } {
        return x;
    }
    let short = is_overlapping_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx || { aaaaaaaaaaaaaaa };
    is_overlapping || { let d = compute_distance(a, b); d < threshold_value_and_something_longer }
}
