fn main() {
    let x: Map<FirstTypeNameAaaaaaaaaaaa, SecondTypeNameAaaaaaaaaaaa, ThirdTypeNameAaaaaaaaaa> = y;
    let x: Map<FirstTypeNameAaaaaaaaaaaaaaaaaaaa, SecondTypeNameAaaaaaaaaaaaaaaaaaaa, ThirdTypeNameAaaaaaaaaaaaaaa> = y;
    let x: Outer<FirstTypeNameAaaaaaaaaaaaaaa, Inner<SecondTypeNameAaaaaaaaaaaaaaa, ThirdTypeNameAaaaaaaaa>> = y;
    let x: Outer<Inner<FirstTypeNameAaaaaaaaaaaaaaa, SecondTypeNameAaaaaaaaaaaaaaa, ThirdTypeNameAaaaaaa>> = y;
    let f: Box<Fn(FirstTypeNameAaaaaaaaaaaaaaaaaaaa, SecondTypeNameAaaaaaaaaaaaaaaaaaaa, ThirdTypeNameAaaaaaaaaaaa)> = g;
}
// Parameters break before a type does, however many lines it costs
fn many_parameters(first: u32, second: u32, third: u32, fourth: u32, fifth: u32, sixth: u32) -> Vec<u32> {
    first
}
fn many_parameters_eight(first: u32, second: u32, third: u32, fourth: u32, fifth: u32, sixth: u32, seventh: u32, eighth: u32) -> HashMap<String, Vec<u32>> {
    first
}
