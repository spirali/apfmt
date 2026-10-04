enum AllSmall {
    A { x: u32 },
    B { x: u32, y: String },
    C {},
    D(u32),
}
enum OneBig {
    A { x: u32 },
    B { first_field_name_long: u32, second_field: u8 },
    C {},
    D(u32),
}
enum OwnLineComment {
    A { x: u32 },
    B {
        // keeps B vertical
        x: u32,
    },
}
enum BrokenTupleVariant {
    A { x: u32 },
    B(FirstFieldTypeName, SecondFieldTypeName, ThirdFieldTypeNameXX),
}
enum OnlyTupleVariants {
    A(u32),
    B(FirstFieldTypeName, SecondFieldTypeName, ThirdFieldTypeNameXX),
}
enum TooWideForMaxWidth {
    VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV { first: u32, second: u32 },
    B { x: u32 },
}
