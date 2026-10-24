// An own-line comment in front of `} else` keeps the indentation of `}` if it is written there: it
// introduces the `else` branch; the first such comment decides for all of them

fn f() {
    if self.with(|| self.tick()) {
        cx.waker().wake_by_ref();
        Poll::Pending

    // Safety: called from the thread that owns `LocalSet`. Because
    // `LocalSet` is `!Send`, this is safe.
    } else if unsafe { self.context.shared.local_state.owned_is_empty() } {
        Poll::Ready(())
    // Otherwise we are not done
    } else {
        Poll::Pending
    }
    if a {
        b();
        // At the indentation of the block, it stays in it
    } else {
        c();
    }
    if a {
        b();
  // Indented less than `}`
    } else {
        c();
    }
    if a {
        b();
        // block level first
    // then brace level
    } else {
        c();
    }
    if a {
        b();
    // brace level first
        // then block level
    } else {
        c();
    }
    if a {
        b(); // trailing
    // brace level
    } else {
        c();
    }
    if a {
        b();
    /* block comment */
    } else {
        c();
    }
    let x = if a {
        1
    // in a `let`
    } else {
        3
    };
    if a {
    // only a comment
    } else {
        c();
    }
}

// Not in front of another `}`
fn g() {
    if a {
        b();
    // before `}` without `else`
    }
    loop {
        b();
    // end of loop
    }
    {
        b();
    // end of block
    }
    match x {
        A => {
            b();
        // before the next arm
        }
        B => {}
    }
    b();
// end of fn
}
