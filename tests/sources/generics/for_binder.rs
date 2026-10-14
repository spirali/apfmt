fn f<F: for<'a> Fn(&'a str) -> &'a str>(f: F) { f("") }
fn g<T, F>(t: T) where for<'a> &'a T: IntoIterator, for <'a,'b> F: Fn(&'a u8, &'b u8) { t }
fn q<T: ?Sized + for<'a> Trait<'a>>() { a }
impl<T> X for T where for<'a> T: Trait<'a> { fn a() {} }
type P = for<'a> fn(&'a u8);
type Q = Box<dyn for<'a> Fn(&'a u8) + Send>;
fn m() {
    let c = for<'a> |x: &'a u8| -> &'a u8 { x };
    let d = for<'a> move |x: &'a u8| x;
}
fn long_where<F>(f: F) where for<'aaaaaaaaaaaa, 'bbbbbbbbbbbbbbb, 'cccccccccccccc> F: Fn(&'aaaaaaaaaaaa u8, &'bbbbbbbbbbbbbbb u8) -> &'cccccccccccccc u8 { a }
fn long_bound<F: for<'aaaaaaaaaaaa, 'bbbbbbbbbbbbbbb, 'cccccccccccccc> Fn(&'aaaaaaaaaaaa u8, &'bbbbbbbbbbbbbbb u8) -> &'cccccccccccccc u8>(f: F) { a }
