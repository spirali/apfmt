fn chains() {
    let a = aaaa.bbbbbbbb.ccccccccc();
    let foo = bar.baz?.quxxxxxxxxxxxxxxxxxxxxxxxxxxx().quuuuuuuuuuuuuuuuuuuuuuuuuuux().aaaaaaaaaaaaaaa();
    x.baz?.quxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx().quuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuux();
    self.pre_comment.as_ref().map_or(false, |c| c.starts_with("//")).then_some(aaaaaaaaaaaaaaaaaaaa);
    x.y.z.quxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx().quxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx();
    let a = aaaaaaaaaaaaaa.bbbbbbbbbbbbb.cccccccccccccc().ddddddddddddd().eeeeeeeeeeeeeeeeeeeeee();
    let a = aaaaaaaaaaaaaa.bbbbbbbbbbbbb().ccccccccccccc().dddddddddddddd()[0].eeeeeeeeeeeeeeeeeeeeeeee();
    let v = items.iter().map(|x| x.len()).collect::<Vec<_>>();
    let v = aaaaaaaaaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbb.collect::<Vec<_>>().await?.cccccccccccc;
    x.await.foo().await?;
    let t = pair.0.1;
}

fn single_element() {
    // `chain_width` limits chains of two or more elements
    builder.push(format_type_bound_list(fmt, node.type_bound_list().unwrap()));
    std::iter::successors(token.next_token(), |t| t.next_token()).find(|t| !t.kind().is_trivia())
}

fn last_element_breaks() {
    let v = foo.bar(aaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccc);
    let v = foo.bar().baz(aaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccc);
    let v = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.bar().baz(aaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbb);
    items.iter().filter(|x| x.is_some()).map(|x| { let y = x + 1; y * 2 });
    items.iter().map(|x| { let y = x + 1; y * 2 }).collect::<Vec<_>>();
    let x = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.b().c(|x| { a; b });
}

fn heads() {
    if aaaaaaaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb().cccccccccccccccccccccccccccc() {
        x
    }
    if aaaaaaaaaaaaaaaa.bbbbbbbb().is_some_and(|x| { let y = x; yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy(y) }) {
        x
    }
    if !rest.split('\n').all(|line| { is_blank(line) || (line.len() >= col && line.as_bytes()[..col].iter().all(|c| *c == b' ')) }) {
        return None;
    }
}

fn comments() {
    a // c
        .b() /* d */
        // e
        .c();
}

fn let_else() {
    let Some(stmt_list) = node.first_child().filter(|c| c.kind() == SyntaxKind::STMT_LIST_LONG_NAME) else {
        return false;
    };
    let Some(x) = foo.bar(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb) else {
        return;
    };
}
