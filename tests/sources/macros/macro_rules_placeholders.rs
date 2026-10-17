// rustfmt copies a whole definition when one of its rules does not parse; afmt formats the rules
// that parse, also with `vis` and `lifetime` metavariables
macro_rules! c {
    // first
    ($a:ident) => {
        $($a)* struct  X;
    }; // trailing

    ($v:vis $n:ident) => { $v struct $n; };
    ($l:lifetime) => { fn f<$l>(x: &$l u8) {} };
    ($x:expr) => { m! { $x  ,   1 }; n!($x  ,   2) };
    ($x:expr) => {
        // comment
        $x  . foo () /* c */
    };
    ($x:expr, $y:ident) => { $x$y };
    ($x:block) => { fn f() $x };
    () => {};
}

fn main() {
        macro_rules! inner {
            ($x:expr) => {{ let  y = "a
  b"; $x + y }};
        }
}

// A definition in a transcriber: the metavariables of both are placeholders
macro_rules! outer {
    ($lhs:expr) => {{
        macro_rules! inner {
            ($T:ty) => {{ let  v: &$T = $lhs.get(); n!($lhs,  v) }};
        }
        inner!(u8)
    }};
}
