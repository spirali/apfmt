// Blank lines between parameters are kept (at most one), as between struct fields; a list on one
// line drops them
fn long_function(
    first: u32,

    second: u32,
    // comment
    third: u32,


    fourth: u32, // trailing

    /* block */ fifth: u32,
) -> u32 {
    1
}

fn short(
    a: u32,

    b: u32,
) {
}

trait T {
    fn method(
        &self,

        first_parameter_xxxxxxxxxxxxxxx: u32,
        second_parameter_yyyyyyyyyyyyyyyyyyy: u32,
    );
}

type F = fn(
    first_parameter_xxxxxxxxxxxxxxx: u32,

    second_parameter_yyyyyyyyyyyyyyyy: u32,
    third_parameter_zzzzzzzzzzzz: u32,
);
