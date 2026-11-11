macro_rules! a {
    ( $x:expr ) => { $x+1 };
    ($x:expr, $y:expr) => {{ $x  + $y }};
}
