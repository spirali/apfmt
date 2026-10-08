fn f() {
    let f = |x| match x { A => 1, B => 2 };
    let f = || Foo { field1, field2: 0, field3: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa };
    let items = list_items(meta.syntax(), |node| {
        match CfgPredicate::cast(node.clone()) {
            Some(predicate) => Some(format_cfg_predicate(fmt, predicate)),
            None => Meta::cast(node).map(|m| format_meta(fmt, m)),
        }
    });
}
