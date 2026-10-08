fn f() {
    if a && let Some(b) = foo() { x }
    if !from_hir_call && let Some(p) = parent { x }
    if let Some(b) = foo() && a { x }
    if foo() && let Some(b) = bar { x }
    while let Some(x) = it.next() && x > 0 { y }
    let v = foo(match x { A => 1, B => 2 });
    match x { Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(x) if some_long_condition_name(x) => {} _ => {} }
}
