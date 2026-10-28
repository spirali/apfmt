// With `match_block_trailing_comma`, a block body is followed by a comma too, also an empty block,
// a block after a broken guard and the last arm; the other bodies have one anyway
fn main() {
    match lorem {
        Lorem::Ipsum => {
            println!("ipsum");
        }
        Lorem::Dolor => println!("dolor"),
        Lorem::Unsafe => unsafe {
            f();
        }
        Lorem::Match => match x {
            A => 1,
            B => 2,
        },
        Lorem::Closure => |x| {
            f(x);
        },
        Lorem::Comment => {
            f();
        } // trailing
        Lorem::Very_long_pattern_name_xxxxxxxxxxxxxxxxxxxxx if some_condition_that_is_long_yyyyyyyyyyyyy => {
            f();
        }
        Lorem::Very_long_pattern_name_xxxxxxxxxxxxxxxxxxxxx if some_condition_that_is_long_yyyyyyyyyyyyy => {}
        Lorem::Kept => {
            g();
        },
        _ => {}
    }
}
