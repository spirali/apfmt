enum Empty {}
enum Unit { A, B, C }
pub enum Public{A,B,}
enum Tuple { A(u32), B(u32, String), C() }
enum Struct { A { x: u32 }, B { x: u32, y: String }, C {} }
enum Discriminant { A = 1, B = 2 + 3, C }
enum BlankLines {
    A,


    B,

    C,
}
enum StructWidth {
    Fits { first_field_name: u32, secondxx: u8 },
    Over { first_field_name: u32, secondxxx: u8 },
    LongerVariantNameFits { first_field_name: u32, secondxx: u8 },
}
enum TupleWidth {
    Fits(FirstFieldTypeName, SecondFieldTypeName, ThirdFieldTypeNameX),
    Over(FirstFieldTypeName, SecondFieldTypeName, ThirdFieldTypeNameXX),
}
