fn main() {
    if replaying {
        for event in all_events {
            let request = Request {
                kind: res_kind_groups(&[vec!["0", "1", "2", "4"], vec!["10", "11", "12", "14", "15"]]),
                name: "request",
            };
        }
    }
    let chart = Chart::new(datasets)
        .labels(vec![format_time_hms(range.start()), format_time_hms(range.end())])
        .style(style);
    let map = ResourceIdMap::from_vec(vec!["cpus".to_string(), "gpus".to_string(), "mem".to_string()]);
}
