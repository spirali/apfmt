// Macro calls whose arguments parse as expressions are laid out as calls (`()`) or arrays (`[]`);
// other arguments and `{}` are kept as they are in the source
fn f() {
    println!(  "{}",   x  );
    let v = vec![1,2,3];
    let v = vec![aaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccc, ddddd];
    let v = vec![aaaaaaaaaaaaaa(1), bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb(2), ccccccccccccccccccccccc(3), d];
    let v = vec![0;   n];
    foo!{ a b c };
    foo! { a  =>  b };
    m!(a => b,   c);
    m!(a,   b,);
    m!(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccc,);
    m!(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccc);
    let x = matches!(e, Expr::ClosureExpr(_) | Expr::MethodCallExpr(_));
    let x = matches!(eeeeeeeeeeeeeeeeeeeeeeeeeeeee, Expr::ClosureExprrrrrrr(_) | Expr::MethodCallExpr(_));
    write!(f, "{}", x)?;
    m!();
    m![];
    m!(a /* c */, b); // after
    assert!(matches!(xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx, Some(yyyyyyyyyyyyyyyyyyyyyyyyyyyyy) | None));
    foo(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, vec![bbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccccccccccc]);
}

foo!(a,   b);
bar! {
    struct X;
}
