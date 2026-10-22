// Blank lines between the fields of a struct pattern are removed, as in a struct literal (rustfmt
// keeps them)
fn f(event: Event) {
    let Event {
        window_identifier,
        client_horizontal_position,
        modifier_shift_pressed,
        modifier_control_pressed,
        ..
    } = event;
    match event {
        Event::Click {
            window_identifier,
            client_horizontal_position,
            ..
        } => {}
        Event::Other { a, b } => {}
    }
}
