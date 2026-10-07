fn f() {
    builder.push(fmt.concat(fmt.text("aaaaaaaaaa"), fmt.nest(fmt.indent, fmt.concat(fmt.text("bbbbbbbbbb"), fmt.text("cccccccccc")))));
    foo(items.iter().map(|x| { let y = x + 1; y * 2 }));
    foo(a.b(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb));
    foo(a.b(c).d(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb));
    foo(a.b(c));
}
