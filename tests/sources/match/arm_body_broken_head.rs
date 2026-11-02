// A head broken anyway follows the braces of the source, as in rustfmt
fn broken_anyway() {
    match x {
        Aaaa => match compute_something_with_a_long_name(argument_number_one, argument_number_two, argument_three) {
            _ => 1,
        },
        Bbbb => {
            match compute_something_with_a_long_name(argument_number_one, argument_number_two, argument_three) {
                _ => 1,
            }
        }
        Cccc => match self.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb().ccccccccccccccccccccccccccc() {
            _ => 1,
        },
        Dddd => {
            match self.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb().ccccccccccccccccccccccccccc() {
                _ => 1,
            }
        }
        Eeee => match (aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb) {
            _ => 1,
        },
        Ffff => match (|| {
            1
        })() {
            _ => 1,
        },
        Gggg => match compute_something_with_a_long_name(argument_number_one, argument_number_two, argument_three) {
            // A comment inside
            _ => 1,
        },
        Hhhh if guard => match compute_something_with_a_long_name(argument_number_one, argument_number_two, argument_three) {
            _ => 1,
        },
    }
}
