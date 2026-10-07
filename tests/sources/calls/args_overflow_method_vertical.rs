// A chain broken before every `.` is not combinable (Style Guide); rustfmt keeps it on the line of `(`
fn f() {
    foo(aaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb().cccccccccccccccccc().ddddddddddddddddddd());
    let x = foo(self.pre_comment.as_ref().map_or(false, |c| c.starts_with("//")).then_some(aaaaaaaaaaaaaaaa));
}
