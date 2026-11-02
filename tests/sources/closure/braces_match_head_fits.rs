// The braces go only where `head {` fits on the line of the parameters
fn fits_on_the_line() {
    let f = |x| {
        match x {
            A => 1,
            B => 2,
        }
    };
    g(|x| {
        match x {
            A => 1,
            B => 2,
        }
    });
    g(a, |x| {
        match x {
            A => 1,
            B => 2,
        }
    });
}

// A comment directly in the braces keeps them
fn comment_in_braces() {
    g(|x| {
        // c
        match x {
            A => 1,
            B => 2,
        }
    });
}

// Without braces in the source there is nothing to remove
fn without_braces() {
    let f = |x| match x {
        A => 1,
        B => 2,
    };
    g(|x| match x {
        A => 1,
        B => 2,
    });
    g(a, |x| match x {
        A => 1,
        B => 2,
    });
}

// The head does not fit on the line of the parameters, and breaks in the braces
fn head_breaks_in_braces() {
    g(|x| {
        match compute_something_long(first_argument_value, second_argument_value, third_argument_value, fourth) {
            A => 1,
            B => 2,
        }
    });
    g(a, |x| {
        match compute_something_long(first_argument_value, second_argument_value, third_argument_value, fourth) {
            A => 1,
            B => 2,
        }
    });
}

// `head {` ends at column 100, then 101, in the braces of the source and without them
fn head_at_max_width() {
    g(|x| {
        match aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(b) {
            A => 1,
        }
    });
    g(|x| {
        match aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(b) {
            A => 1,
        }
    });
    g(|x| match aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(b) {
        A => 1,
    });
    g(|x| match aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(b) {
        A => 1,
    });
}

// A match arm body: `head {` ends at column 100, then 101
fn arm_body_at_max_width() {
    match q {
        A => match aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(b) {
            B => 1,
        },
        A => match aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(b) {
            B => 1,
        },
    }
}
