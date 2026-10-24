fn struct_pattern(notification: ProgressNotification) {
    match notification {
        ProgressNotification::Packing { stage, current, total } => match stage {
            PackBuilderStage::AddingObjects => RemoteProgressState::PackingAddingObject,
            PackBuilderStage::Deltafication => RemoteProgressState::PackingDeltafiction,
        },
        _ => RemoteProgressState::Done,
    }
}

fn tuple_struct_pattern(value: Value) {
    match value {
        Value::Something(first_long_argument, second_long_argument, third_long_argument) => match first_long_argument {
            Kind::A => 1,
            Kind::B => 2,
        },
        _ => 0,
    }
}

fn short_pattern(value: Value) {
    match value {
        Short { a } => match kind {
            Kind::A => 1,
            Kind::B => 2,
        },
        _ => 0,
    }
}

fn block_avoids_broken_pattern(value: Value) {
    match value {
        Value::Something { first, second } => match some_rather_long_discriminant_name {
            Kind::A => 1,
            Kind::B => 2,
        },
        _ => 0,
    }
}

fn or_pattern(value: Value) {
    match value {
        Value::FirstVeryLongVariantName | Value::SecondVeryLongVariantName | Value::Third => match kind {
            Kind::A => 1,
            Kind::B => 2,
        },
        Value::FirstVeryLongVariantName | Value::SecondVeryLongVariantName | Value::Third => Struct {
            a: 1,
            b: 2,
        },
        _ => 0,
    }
}

fn guard(value: Value) {
    match value {
        Value::Something { first, second, third } if first.is_some_long_condition() => match kind {
            Kind::A => 1,
            Kind::B => 2,
        },
        _ => 0,
    }
}

fn long_head(value: Value) {
    match value {
        Value::Something { first, second, third } => match some_function_with_a_long_name(first_argument, second) {
            Kind::A => 1,
            Kind::B => 2,
        },
        _ => 0,
    }
}

fn no_room_for_brace(value: Value) {
    let res = match value {
        (_, Err(ConfigLoadError::BadConfig(err))) | (Err(ConfigLoadError::BadConfig(errrrr)), _) => {
            return Err(ConfigLoadError::BadConfig(err));
        }
        _ => 0,
    };
}
