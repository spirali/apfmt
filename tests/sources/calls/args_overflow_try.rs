fn main() {
    let x = foo(bar(aaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccc)?);
    let x = foo(bar(aaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccc)??);
    let x = foo(x.bar(aaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccc)?);
    let x = foo(x.y().bar(aaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccc)?);
    let x = foo(Bar { aaaaaaaaaaaaaaaaaaaaaaaaaaa: 1, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb: 2, c: 3 }?);
    let x = foo(&bar(aaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccc)?);
    let x = foo(a, bar(aaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccc)?);
    let x = foo(bar(|x| {
        x + 1
    })?);
    foo(a.b().c()?.d().e(aaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccccccc)?);
    foo(bar(a)?);
}
