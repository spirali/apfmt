fn make_global_settings(opts: CommonOpts) -> GlobalSettings {
    let server_dir = absolute_path(
        opts.server_dir
            .unwrap_or_else(default_server_directory_path),
    );
    if replaying {
        for event in all_events {
            if let Some(job) = jobs.get(&event.job_id) {
                events.push(
                    Event::at(completion_date, EventPayload::JobCompleted(job.id)),
                );
                server_ref.set_client_events(Box::new(
                    UpstreamEventProcessor::new(state.clone(), senders),
                ));
                self.comm.send_message_to_server(
                    FromWorkerMessage::Notify(WorkerNotifyMessage { task_id, message }),
                );
                queue.add_allocation(
                    Allocation::new(id.to_string(), worker_count, PathBuf::new().into()),
                );
            }
        }
    }
}
