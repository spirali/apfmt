// A block around a single expression is removed also when a comment is inside that expression
// (rustfmt keeps the block); a comment directly in the block keeps it
fn f() {
    match k {
        A => {
            match k.code {
                // comment
                C => 1,
                _ => 2,
            }
        }
        B => {
            unsafe {
                buf.extend(1);
                // SAFETY: x
                validity.push(false)
            }
        }
        C => {
            self.last_non_null().and_then(|idx| {
                // SAFETY: in bounds
                unsafe { self.get_unchecked(idx) }
            })
        }
        D => {
            // comment in the block
            g(1)
        }
        E => { foo(a, /* c */ b) }
        F => { foo(a, b) } // after
        G => {
            {
                x.map(|v| {
                    // inside closure
                    v + 1
                })
            }
        }
    }
}
