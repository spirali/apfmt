struct Unit;
struct Empty {}
struct EmptyTuple();
struct Named { a: u32, b: String }
pub struct Public {
    pub a: u32,
    pub(crate) b: &'static str,
    c: Vec<u32>
}
struct Spaced{a:u32,b : HashMap<String,u32>,}
struct BlankLines {
    a: u32,


    b: u32,

    c: u32,
}
struct Tuple(u32, String);
pub struct PublicTuple(pub u32, pub(crate) String);
struct FitsFnCallWidth(FirstFieldTypeName, SecondFieldTypeName, ThirdFieldTypeNameX);
struct OverFnCallWidth(FirstFieldTypeName, SecondFieldTypeName, ThirdFieldTypeNameXX);
