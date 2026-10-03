fn main() {
    if x {
        // only comment
    }
    if x { f(); } // trailing
    else { g(); }
    if x {
        f();
    }
    // before else
    else {
        g();
    }
    if x { // after brace
        f();
    } else { // after else brace
        g();
    }
    if /* c */ x /* d */ {
        f();
    } /* e */ else /* f */ {
        g();
    }
    let a = if x { 1 } /* c */ else { 2 };
    let a = if x { /* c */ 1 } else { 2 };
}
