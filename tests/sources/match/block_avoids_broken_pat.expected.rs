fn or_pattern_fits_with_block(expr: AExpr) {
    match expr {
        AnonymousFunction { input, .. } | Function { input, .. } | AnonymousAgg { input, .. } => {
            container.extend(input.iter().map(|e| e.node()))
        }
        _ => {}
    }
}

fn short_pattern_not_broken_for_body(value: Value) {
    match value {
        Type::Path(of) => {
            match parse_type_path(position, attrs.clone(), TypePathContext::Ptr, of)? {
                Kind::A => 1,
                Kind::B => 2,
            }
        }
        Type::Path(of) => match value {
            Kind::A => log::debug!(
                "Allocation {allocation_id} marked externally as running, even though a worker has not connected yet"
            ),
            Kind::B => 2,
        },
        _ => 0,
    }
}
