fn main() {
    if worker_time_limit > queue_time_limit {
        return Err(anyhow::anyhow!("Worker time limit cannot be larger than queue time limit"));
    }
    let directives = match mode {
        DirectivesMode::Stdin => Some(parse_hq_directives(stdin.expect("Stdin directives is not present"))?),
        ((hours, Some(minutes)), Some(seconds)) => Ok(Duration::from_secs(hours as u64 * 3600 + minutes as u64 * 60 + seconds as u64)),
        Ok(allocation) => ToClientMessage::AutoAllocResponse(AutoAllocResponse::AllocationInfo(allocation)),
        _ => None,
    };
    let input = match value {
        StdioValue::Map(map) => {
            if map_is_valid {
                Ok(StdioDefInput::Full(StdioDefFull::deserialize(serde::de::value::MapAccessDeserializer::new(map))?))
            } else {
                None
            }
        }
    };
    let nodes = parse_range(&std::fs::read_to_string("/sys/devices/system/node/possible")?)?;
    self.stats = Some(generate_time_data(steps, |time| get_task_counts_for_job(job_id, data, time)));
    process_the_request(RequestWithLongName { first_field_name, second_field_name });
    register_the_numbers(&[first_number_value, second_number_value, third_number_value_xx]);
}
