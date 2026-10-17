macro_rules! b {
    ($($x:expr),* $(,)?) => { vec![$($x),*] };
}

mod m {
    fn f() {
        macro_rules! d {
            ($a:ident) => {
                $($a)*  x = 1;
            };

            ($a:ident, $b:expr) => [
              $($a)* = "a
  b"
            ];
        }
    }
}

macro_rules! e {
    ($a:expr) => { $($a)* };
    ($a:expr) /* c */ => /* d */ { $($a)* };
}

macro_rules! not_rules {
    ($a:expr) => $a;
}
