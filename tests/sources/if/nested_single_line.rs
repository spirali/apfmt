fn main() {
    let v = if a { if b { 1 } else { 2 } } else { 3 };
    let v = if a { if b { if c { 1 } else { 2 } } else { 3 } } else { 4 };
    let v = if a { if b { if c { first_value } else { second_value } } else { third_value } } else { fourth_value };
}
