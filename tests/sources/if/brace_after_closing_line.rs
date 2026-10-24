fn hugged_right_operand() {
    if e == Err(raft::Error::Store(StorageError::Other(format!("region {} is not found", region_id)))) {
        x();
    }
}

fn array_of_struct_literal() {
    if a == &[actions::Action::SwitchToMode { input_mode: InputMode::Normal, other: something_long }] {
        Some(k)
    }
}

fn closure_then_operator() {
    if field.metadata.is_some_and(|md| {
        md.contains_key(DTYPE_ENUM_VALUES_LEGACY) || md.contains_key(DTYPE_ENUM_VALUES_NEW_LONGER_NAME)
    }) && matches!(key_type, IntegerType::UInt8 | IntegerType::UInt16 | IntegerType::UInt32) {
        x();
    }
}

fn chain_after_broken_root() {
    match LayoutApplier::new(&self.viewport, &self.senders, &self.sixel_image_store, &self.link_handler)
        .apply_layout(layout, floating_panes_layout, new_terminal_ids, new_floating_terminal_ids)
    {
        Ok(x) => x,
        Err(e) => e,
    }
}

fn while_hugged() {
    while queue.pop_front_if_matching(Request::Connect { address: some_address, port: some_port_value }) {
        x();
    }
}

fn broken_before_operator() {
    if some_condition_that_is_long && another_condition_that_is_long && call_with_args(first_argument, second) {
        x();
    }
}

fn operator_after_closing() {
    if call_with_long_arguments(first_argument, second_argument, third_argument, fourth_arg) == expected {
        x();
    }
}

fn element_after_closing() {
    match LayoutApplier::new(&self.viewport, &self.senders, &self.sixel_image_store, &self.link_handlerr).apply() {
        Ok(x) => x,
        Err(e) => e,
    }
}

fn let_else() {
    let Some(x) = y.foo("abc", fairly_long_identifier, "def", "123456", "string", "cheese", "more") else {
        return;
    };
}

fn first_operand_fits() {
    if pane_geom_is_big_enough(&geom_with_current_offset) && pane_geom_is_unoccupied_and_inside_viewport(viewport, &geom_with_current_offset, other_geoms) {
        x();
    }
}

fn let_chain_first_operand_fits() {
    if looks_like_bare_specifier(path) && let Some(url) = resolver.resolve_bare_specifier_in_npm_snapshot(path, Some(&referrer), import_kind_to_resolution_mode(kind)) {
        x();
    }
}
