// A closure or a block after `&` / `&mut` overflows as the last of more arguments

fn closures() {
    iter(depth, &mut |expr| {
        f(expr);
        g(expr);
    });
    visit_exprs(node, depth + 1, &mut move |expr| {
        if let Some(x) = expr.method() {
            count += x;
        }
    });
    callback(&state, &|event: Event| {
        log(event);
        handle(event);
    });
    nested(a, &&mut |x| {
        f(x);
        g(x);
    });
}

fn blocks() {
    s.field("deps", &{
        let deps = self.deps.lock();
        deps.iter().map(|d| d.name()).collect::<Vec<_>>()
    });
    s.field("entries", &mut unsafe {
        let ptr = self.ptr.as_ref();
        ptr.entries()
    });
}

fn not_overflowing() {
    too_long_for_the_first_line(first_argument_value, second_argument_value, &mut |expr| {
        f(expr);
    });
}
