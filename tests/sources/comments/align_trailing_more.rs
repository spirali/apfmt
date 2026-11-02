const A: [u8; 7] = [
    0, 0, 0, // len
    4, // kind
    0, 0, 0, 0, // id
];

struct S {
    /// doc
    cc: u32, // c
    dddddddd: Vec<u32>, // d
}

fn g() {
    let x = [
        ((Direction::Forward, 1usize), (0, 1)), // T|his is a simple alphabetic line
        ((Direction::Forward, 999usize), (0, 32)), // This is a simple alphabetic line|
    ];
}
