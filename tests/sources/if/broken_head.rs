// A broken control-line head costs a split, as `{` on its own line does (Style Guide: the control
// line is broken only when it does not fit)
fn f() {
    if a {
        // 1. let x = if <chain>, 104 wide
        let allocation_state = if worker_authentication(&core_ref2, socket, value_xxxxxxxxxxxxx).await {
            1
        } else {
            2
        };
        // 2. assignment x = if <chain>, 104 wide
        self.allocation_state = if worker_authentication(&core_ref2, socket, value_xxxxxxxxxx).is_ok() {
            1
        } else {
            2
        };
        // 3. if <call>: head fits, ` {` overflows (101)
        if worker_authentication_has_succeeded(&core_ref2, socket_address, value_xxxxxxxxxxxxxxxxx) {
            x
        }
        // 4. if <call>: head does not fit (110)
        if worker_authentication_has_succeeded(&core_ref2, socket_address, value_xxxxxxxxxxxxxxxxxxxxxxxxxx) {
            x
        }
        // 5. while <chain>: ` {` overflows (101)
        while connection_state.worker_authentication(&core_ref2, value_xxxxxxxxxxxxx).await.is_ok() {
            x
        }
        // 6. if <method call>: ` {` overflows (101)
        if connection_state.is_authenticated_with(&core_ref2, socket_address, value_xxxxxxxxxxxxxx) {
            x
        }
        // 8. if a && b: ` {` overflows (101)
        if worker_authentication_has_succeeded(&core_ref2, socket) && value_xxxxxxxxxxxxxxxxxxxxxxx {
            x
        }
        // 9. match a && b: ` {` overflows (101)
        match worker_authentication_has_succeeded(&core_ref2, socket) && value_xxxxxxxxxxxxxxxxxxxx {
            true => x,
            false => y,
        }
        // 10. let x = match <call>, 103 wide
        let allocation_state = match worker_authentication(&core_ref2, socket, value_xxxxxxxxxxxxxxx) {
            Ok(r) => r,
        };
        // 11. let x = match <chain>, 110 wide
        let allocation_state = match connection.worker_authentication(&core_ref2, value_xxxxxxxxxxxxx).await {
            Ok(r) => r,
        };
        // 12. let x = if <call>, 110 wide
        let allocation_state = if worker_authentication_has_succeeded(&core_ref2, value_xxxxxxxxxxxxxxxxxxx) {
            1
        } else {
            2
        };
        // 13. `if let` with the expression on the line of `=` and `{` on its own line
        if let Some((resource_id, exact)) = exact_single_resource_gap(h_rqv, resources, &occupants) {
            free.set(resource_id, exact);
        }
    }
}
