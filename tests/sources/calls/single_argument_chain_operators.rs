fn chains_with_operators() {
    let t1 = rt.new_task(&TaskBuilder::new().cpus(8).next_variant().cpus(2).add_resource(1, 2));
    let t2 = rt.new_task(&mut TaskBuilder::new().cpus(8).next_variant().cpus(2).add_resource(1, 2));
    let up1 = task_finished(*rt.worker(w1).sn_assignment().unwrap().assigned_tasks.iter().next().unwrap());
    assert!(!worker.sn_assignment().unwrap().prefilled_tasks.contains(task_id).and_more_stuff());
    let total = compute_total(self.workers.iter().filter(|w| w.is_active()).map(|w| w.cpus).sum() as u32);
    let delta = compute_delta(-self.workers.iter().filter(|w| w.is_active()).map(|w| w.cpus).sum::<i64>());
    consume(&mut *self.state.borrow_mut().workers.get_mut(&worker_id).unwrap().running_tasks);
}

fn chains_still_overflowing() {
    let nodes = parse_range(&std::fs::read_to_string("/sys/devices/system/node/possible")?)?;

    let count = process(&tasks.iter().map(|task| task.resources.cpus.amount_of_units_needed()).collect());
}

pub async fn submit_computation_from_job_file(opts: JobSubmitFileOpts) -> anyhow::Result<()> {
    let jdef =
        {
            JobDef::parse(&std::fs::read_to_string(&opts.path).map_err(|e| {
                anyhow::anyhow!(format!("Cannot read {}: {}", opts.path.display(), e))
            })?)?
        };
    send_submit_request(jdef).await
}
