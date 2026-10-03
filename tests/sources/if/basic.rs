fn main() {
    if x { f(); }
    if x { f(); } else { g(); }
    if x { f() } else if y { g() } else { h() }
    if   f(1,2)   {
        g();
    }
    else
    {
        h();
    }
    if !x { f(); }
    if (a == b) { f(); }
    if a == b && c != d { f(); }
    if x { if y { f(); } }
    if x { f(); };
}
