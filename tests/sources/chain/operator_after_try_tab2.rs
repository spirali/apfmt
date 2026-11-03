// With two spaces, `})?` ends after the next indentation and breaks, while `})` and `)?` stay
fn f() {
  let a = foo(|x| {
    let y = x + 1;
    y
  })? * 2;
  let a = foo(|x| {
    let y = x + 1;
    y
  }) * 2;
  let a = foo(
    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
    ccccc,
  )? * 2;
}
