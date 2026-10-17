// An index that does not fit on one line is block indented between the brackets, never broken before
// `[` (Style Guide "Array accesses, indexing, and slicing"); rustfmt breaks before `[` and hugs a call
fn f() {
    let refs = &mut *values[aaaaaaaa + bbbbbbbb * cccccccc - dddddddd][
        eeeeeeee * ffffffff + gggggggg
    ][hhhhhhhh - iiiiiiii * jjjjjjjj + kkkkkkkk];
    let ranges =
        &slice[start_index + offset * stride..end_index - offset * stride + remainder_of_division];
    let value = a_long_target_expression_name[a_long_indexing_expression_name + another_long_one];
    let value =
        a_long_target_expression_name.method_call()[a_long_indexing_expression_name + another_long];
    let value = self.aaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbb[index]
        .cccccccccccccccccccccccc()
        .ddddddd();
    let value = matrix[
        compute_row_index(
            aaaaaaaaaaaaaaaa,
            bbbbbbbbbbbbbbbbbbb,
            cccccccccccccccccc,
            dddd,
        )
    ];
    foo[42];
    x[i] =
        yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy;
    let x = // c
        a[b];
    let x = a[ // c
        b
    ];
    let x = a[
        b // c
    ];
}
