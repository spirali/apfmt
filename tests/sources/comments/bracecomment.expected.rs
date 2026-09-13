fn short() { // Fits on the line with the brace
    call();
}

fn long() {
    // This comment is too long to stay on the line with the opening brace, so it has to be moved
    call();
}

fn block() { /* Block comment */
    call();
}
