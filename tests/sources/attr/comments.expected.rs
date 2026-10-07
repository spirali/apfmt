/// Doc comment
#[derive(Debug)] // trailing comment
// own-line comment between attributes
#[repr(C)]
/* block */ #[allow(unused)]
struct S;

#[cfg(test)]

mod tests {}

#[derive(Debug /* inside */, Clone)]
struct T;
