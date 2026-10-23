fn arm_body_with_braces() {
    let x = match reason {
        AllocationSyncReason::AllocationExternalChange(status) => {
            match (&mut allocation.status, status) {
                (AllocationState::Queued { .. }, AllocationExternalStatus::Running) => {
                    log::debug!(
                        "Allocation {allocation_id} marked externally as running, even though a worker has not connected yet"
                    );
                    None
                }
                _ => None,
            }
        }
    };
}

fn arm_body_without_braces() {
    let x = match reason {
        AllocationSyncReason::AllocationExternalChange(status) => {
            match (&mut allocation.status, status) {
                _ => None,
            }
        }
        Short(status) => match (&mut allocation.status, status) {
            _ => None,
        },
    };
}

fn after_let() {
    if a {
        if b {
            if c {
                let (connection, message) =
                    match worker_authentication(&core_ref2, socket, address).await {
                        Ok(r) => r,
                        Err(e) => return,
                    };
            }
        }
    }
}

fn after_assignment() {
    if a {
        if b {
            if c {
                if d {
                    if e {
                        task.state =
                            match std::mem::replace(&mut task.state, JobTaskState::Waiting) {
                                JobTaskState::Running { started_data } => {
                                    JobTaskState::Finished { started_data }
                                }
                                _ => panic!("Invalid task state"),
                            };
                    }
                }
            }
        }
    }
}

fn statement() {
    match std::mem::replace(
        &mut allocation.state,
        AllocationState::Waiting,
        something_long,
    ) {
        _ => (),
    }
}
