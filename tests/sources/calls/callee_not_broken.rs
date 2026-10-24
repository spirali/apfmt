// "Prefer not to break a line in the callee expression" (Style Guide): the arguments break first

fn f() {
    {
        {
            Some((self.filler)(self.elements_required - self.elements_from_next_back));
            Some((self.a.filler)(self.elements_required - self.elements_from_next_back));
        }
    }
    foo((xs.filler)(self.elements_required - self.elements_from_next_back - aaaaaaaaaaaa));
    let value = (self.filler)(self.elements_required - self.elements_from_next_back - aaaaaaaa);
    let x = (self.filler)(self.elements_required - self.elements_from_next_back, aaaaaaaaaaa);
    make_handler(configuration)(first_argument_value, second_argument_value, third_argument);
}

// A callee that cannot be on one line keeps the arguments on its last line
fn g() {
    let v = (|a: u32| {
        let b = a + 1;
        b * 2
    })(x);
    let v = (match kind {
        Kind::A => handle_a,
        Kind::B => handle_b,
    })(first_argument, second);
    let v = (if flag { handle_a } else { handle_b_with_a_rather_long_name_so_that_it_breaks_aaaa })(x);
    let (rest, value) = (|input| {
        let (input, _) = tag("x")(input)?;
        Ok((input, ()))
    })(input)?;
}
