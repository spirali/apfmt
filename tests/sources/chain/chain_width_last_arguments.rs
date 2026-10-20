// A chain on one line with the arguments of its last element broken stays so only if its first line,
// the start of the arguments included, is within `chain_width` (as rustfmt measures it)
fn f() {
    if a {
        let resource_id = self.resource_names.get_mut(name).copied().unwrap_or_else(|| {
            let resource_id = self.resource_names.len() as ResourceId;
            resource_id
        });
        let shared_index = *self.configuration_index.entry(conf.clone()).or_insert_with(|| {
            let index = self.shared_data.len();
            index
        });
        self.query_known_worker_ids_at(time).filter_map(|(id, status)| match status {
            WorkerStatus::Connected => Some(id),
            WorkerStatus::Disconnected(_) => None,
        })
    }
    // Within `chain_width` with the start of the arguments: stays
    items.iter().map(|item| {
        let value = compute_the_value_of_the_item(item);
        value
    });
}
