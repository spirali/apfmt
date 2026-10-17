fn broken_pattern_short_expression() {
    if let JobTaskDescription::Array { task_desc, resource_rq, entries, .. } = &info.submit_data.task_desc {
        show(task_desc);
    }
    while let Some(Message { identifier, payload, sender, .. }) = connection.next_message_now() {
        handle(identifier);
    }
}

fn broken_pattern_broken_expression() {
    if let JobTaskDescription::Array { task_desc, resource_rq, entries, .. } = load_description(first_argument_value, second_argument_value, third_argument_value, fourth_argument) {
        show(task_desc);
    }
    if let Some(description) = tasks.iter().find(|task| task.id == wanted_id && task.is_ready_to_run()) {
        show(description);
    }
}

fn pattern_on_one_line() {
    if let Some(description) = load_the_task_description(first_argument_value, second_argument_value, third) {
        show(description);
    }
    if let Some(description) = self.descriptions.get(&task_id) {
        show(description);
    }
}
