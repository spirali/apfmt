// A single method-call argument is limited only by `max_width`, not by `fn_call_width` (as rustfmt)
fn f() {
    builder.push(fmt.token_with_edges(attr.pound_token().unwrap(), Word, Tight));
    builder.push(Delimiters::parens_of(inner.syntax()).enclose(fmt, path.into_frag()));
    let lines = list.attrs().map(|attr| format_attr(fmt, attr)).chain(items.map(|item| format_item(fmt, Item::cast(item).unwrap())));
}
