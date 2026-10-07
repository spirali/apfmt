fn closures() {
    let f = |x| { x + 1 };
    let f = move |x: i32, y| -> i32 { x + y };
    let f = |x| -> T { x };
    let f = |x| { let y = x; y };
    let f = |x| unsafe { x };
    let f = async move || 1;
    let f = || {};
    let f = |(a, b): (u32, u32), &c| a + b + c;
    let f = |x| x.aaaaaaaaaaaaaaaaaaaaaaaaaaa().bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb().cccccccccccccccccccccc();
    let f = |x| some_long_function_name(aaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbccccccccc, ddddddddddddd);
    let f = |x| [aaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbccccccccc, ddddddddddddddddddddd];
    xxxxxxxxxxxxxxxxxxxxxxxx.map(|x| if aaaaaaaaaaaaaaaa { bbbbbbbbbbbbbbbbbbbbb } else { cccccccccccccccccccc });
}

fn arguments() {
    foo(|param| { action(); foo(param) });
    foo(first_arg, x, |param| { action(); foo(param) });
    foo(aaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, |param| { action(); foo(param) });
    foo(a, |x| x, |param| { action(); foo(param) });
    foo(a, |param| param + 111111111111111111111111111111111111111111111111111111111111111111111111);
    foo(aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, |x| some_long_function_name(aaaaaaaaaaaaaa, bbbbbbbb));
    foo(aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, |x| some_long_function_name(aaaaaaaaaaaaaa, bbbbbbbb), c);
    foo(|x| some_long_function_name(aaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbccccccccc));
    tree.use_tree_list().is_some_and(|list| !has_comments(list.syntax()) && list.use_trees().all(|t| is_empty_tree(&t)))
}
