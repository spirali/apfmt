// The right side of `let` or of an assignment moves to the next line when it saves a line there as
// a chain on a single line except the arguments of its last element, against the chain broken before
// its elements on the line of `=`
fn f() {
    for i in 0..usize::min(edges.inputs().len(), edges.outputs().len()) {
        edges.inputs()[i].projection_state_mut().names = edges.outputs()[i].take_names().map(|mut x| {
            x.clear();
            x
        });
    }
    let maybe_cpu_prof_config_for_workers = cli_options.cpu_prof_dir().map(|dir| {
        let name = dir.file_name();
        CpuProfilerConfig { dir, name }
    });
    let (graph_modules, doc_only_modules): (Vec<_>, Vec<_>) = test_modules.into_iter().partition(|specifier| {
        let module = graph.get(specifier);
        module.is_some()
    });
    // A broken chain on the next line too: it moves only when it saves two lines
    let output = cmd.args(["run", "--release", "--bin", "something_long", "--", "arguments_xxxxxxxx"]).output()?;
    // No line saved: it stays on the line of `=`
    let value = some_receiver.method_one().method_two(|argument_one, argument_two| argument_one + argument_two);
}
