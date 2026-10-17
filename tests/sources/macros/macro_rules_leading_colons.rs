// The transcriber of a rule is formatted as items: a path starting with `::` keeps its space
macro_rules! impl_id {
    ($name:ident, $type:ty) => {
        impl ::std::convert::From<$type> for $name {
            fn from(value: $type) -> Self {
                Self::new(value)
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl<'a> $name {
            fn bytes(&self) -> &'a [u8] {
                &[]
            }
        }
    };
}
