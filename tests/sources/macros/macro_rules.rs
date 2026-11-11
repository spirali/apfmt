// Transcribers with a repetition, which neither rustfmt nor apfmt parses, are copied
macro_rules! define_wrapped_type {
    ($name: ident, $type: ty $(, $visibility: vis)?) => {
        #[derive(::std::clone::Clone)]
        #[repr(transparent)]
        $($visibility)* struct $name($crate::WrappedRcRefCell<$type>);
    };
}

macro_rules! a {
    ($name: ident, $type: ty $(, $vis: vis)?) => {
        struct $name(  $type );
    };
    ( $x:expr ) => { $x+1 };

    // between
    ($x:expr, $y:expr) => { $x  + $y };
    ($x:expr, $y:expr, $z:expr) => {
        let   v = $crate::f($x, $y);
        if v { $z } else { 0 }
    }
}

mod m {
macro_rules! d {
        ($a:ident) => {
            let   x = 1;
        };
}
}

macro_rules! items {
    ($t:ty) => {
        impl Foo for $t { fn foo(&self) -> u32 { 1 } }
    };
}

macro_rules! empty {}

#[macro_export]
macro_rules! paren (
  ($a:expr) => ($a)
);

macro_rules! bracket [
  ($a:expr) => ($a)
];
