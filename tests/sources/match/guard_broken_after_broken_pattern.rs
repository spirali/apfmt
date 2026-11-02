fn f() {
    match x {
        Foo { value, unit, aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccccccccccc } if cond(value) => 1,
        Foo { value, unit, aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccccccccccc } if aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa && bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb && ccccccccccc => { 2 }
        Foo(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccccccccccc) if g(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccc) => { 3 }
        Foo { aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccccccccccc } if xxxxxxx.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa().bbbbbbbbbbbbbbbbbbbbbbbbbbbbb().ccccccccccccccccccccc().dddddddddddddddddd() => { 4 }
        Foo { aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccccccccccc } if let (MaintainOrderJoin::None, true, Some(hive_left), Some(hive_right)) = (&options.args.maintain_order, hive_rewrite_supports_join_type(&options.args.how), is_hive_partitioned(input_left, ir_arena), is_hive_partitioned(input_right, ir_arena)) => { 5 }
        // The pattern fits on a line: it stays there, the guard goes on its own line
        Some(xxxxxxxxxxxxxxxxx) if dtype.is_primitive_numeric() || dtype.is_temporal() || dtype.is_decimal() => {
            g();
        }
        // An or-pattern ends on a `|` line: the guard goes on its own line
        Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(a) | Bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb(b) | Cccccccccccccccccccccccccc(c) if g(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccc) => { 6 }
    }
}
