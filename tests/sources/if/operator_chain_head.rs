// A chain behind `&`, `&mut`, `*`, `!` or a cast in a control-flow head hangs as the chain does: `{`
// on its own line after it (HangingHeadRule)
fn f() {
    if a {
        match &self.allocation_state.lookup_the_worker(worker_identifier).unwrap_or_default() {
            _ => x,
        }
        match &mut self.allocation_state.lookup_the_worker(worker_identifier).unwrap_or_def() {
            _ => x,
        }
        match self.allocation_state.lookup_the_worker(worker_identifier).count_them_all() as u8 {
            _ => x,
        }
        for item in &self.allocation_state.lookup_the_worker(worker_identifier).items_of_them {
            x
        }
        if let Some(x) = &self.allocation_state.lookup_the_worker(worker_identifier).first_one {
            x
        }
        while *self.allocation_state.lookup_the_worker(worker_identifier).is_connected_now() {
            x
        }
        if !self.allocation_state.lookup_the_worker(worker_identifier).is_connected_nowxx() {
            x
        }
    }
}
