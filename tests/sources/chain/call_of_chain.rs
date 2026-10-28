// A call of a chain's result (`a.b()(x)`, `a.handlers[i](x)`) is a part of the chain: its arguments
// follow the last element on its line, as an index does, and a broken argument list is block
// indented from that line; rustfmt indents it from the start of the chain
fn f() {
    let x = self.handlers[index](first_argument_xxxxxxxxxxxxxxxxxx, second_argument_yyyyyyyyyyyyyyyyyyyyyy);
    let x = self.registered_event_handlers_by_kind.handlers_for_this_kind[event_index](first_argument_xxxxxxx, second_argument_yyyyyyyyyyyzzzzzz);
    self.registered_event_handlers_by_kind.handlers_for_this_kind[event_index](first_argument_xxxxxxx, second_argument_yyyyyyyyyyyzzzzzz).unwrap();
    self.registered_event_handlers_by_kind.handlers_for_this_kind[event_index](first_argument_xxxxxxx, second_argument_yyyyyyyyyyyzzzzzz).unwrap().into_inner();
    let x = self.registered_event_handlers_by_kind.make_handler_for_kind()(first_argument_xxxxxxx, second_argument_yyyyyyyyyyyzzzzzz);
    let x = self.registered_event_handlers_by_kind.make_handler()?(first_argument_xxxxxxx, second_argument_yyyyyyyyy)?;
    // Hugged by a call only with the chain on the line of `(`, as a method call
    foo(self.registered_event_handlers_by_kind.handlers_for_this_kind[event_index](first_argument_xxxxxxx, second_argument_yyyyyyyyyyyzzzzzz));
    foo(self.handlers[index](first_argument_xxxxxxxxxxxxxxxxxx, second_argument_yyyyyyyyyyyyyyyyyyyyyy));
    // A broken chain as a head: `{` on its own line
    if self.registered_event_handlers_by_kind.handlers_for_this_kind[event_index](first_argument_xxxxxxx, second_argument_yyyyyyyyy) {
        a();
    }
    // A call of a call's result is not a chain
    let x = make_handler_for(configuration_value)(first_argument_xxxxxxxxxxxxxxx, second_argument_yyyyyyyyyyyy);
}
