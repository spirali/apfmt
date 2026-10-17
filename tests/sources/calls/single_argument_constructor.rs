fn parse_group(span: Span, items: Vec<u32>) -> Result<Group, ParseError> {
    if items.is_empty() {
        return Err(ParseError::custom(span, "Each group has to contain at least a single element"));
    }
    if start > end {
        return Err(crate::Error::GenericError(format!("Task {} is defined multiple times", t.id)));
    }
    let map = Map(InnerMap::with_capacity_and_hasher(capacity, FxBuildHasher::default_value()));
    let result = match outcome {
        Outcome::Submitted(id) => Ok(AllocationSubmissionResult::new(Ok(id.to_string()), dir.into())),
        Outcome::Failed => Err(DsError::AuthenticationRejected(format!("Received error: {}", e.message))),
        Outcome::Pending => Ok(TaskLaunchData { task_future: Box::pin(task_future), serialized_context }),
    };
    let request = match value {
        AllocationValue::Int(value) => {
            match kind {
                Kind::Compact => {
                    match policy {
                        Policy::Default => {
                            let request = match value {
                                AllocationValue::Int(value) => AllocationRequest::Compact(ResourceAmount::new_units_of_resource(value)),
                                AllocationValue::Other => AllocationRequest::Compact(ResourceAmount::from_float_value(value, unit)),
                            };
                        }
                    }
                }
            }
        }
    };
}
