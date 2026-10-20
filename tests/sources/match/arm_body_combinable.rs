fn main() {
    match x {
        Assign => {
            task.state = JobTaskState::Failed {
                error: error.clone(),
                started_data: None,
                end_date: now,
            }
        }
        AssignShort => { a = 1 }
        AddAssign => {
            *self = match self {
                P => Q,
                R => S,
            }
        }
        Ret => {
            return Err(anyhow!(
                "Canceling job {job_id} failed; job not found and something else"
            ));
        }
        Chain => xs
            .iter()
            .map(|x| x.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa())
            .filter(|x| x.bbbbbbbbbbbbbbbbbbb())
            .count(),
        Binary => {
            first_operand_value_xxxxxxxxxxxxxxxxxx + second_operand_value_xxxxxxxxxxxxxxxxxxxxxx
        }
        Call => a_function_call(
            another_call(argument1, argument2),
            another_argument_xxxxxxxxxxxxxxxxxxxx,
        ),
        Struct => JobTaskState::Failed {
            error: error.clone(),
            started_data: None,
            end_date: nowxxxxx,
        },
        Ref => &JobTaskState::Failed {
            error: error.clone(),
            started_data: None,
            end_date: nowxxxxxxxxxx,
        },
        MacroInBlock => {
            log::error!("short")
        }
        MacroInBlockLong => {
            log::error!(
                "Canceling job {job_id} failed; job not found and something else {}",
                x
            )
        }
        Macro => log::error!("short"),
        MacroLong => log::error!(
            "Canceling job {job_id} failed; job not found and something else {}",
            x
        ),
        Json => {
            json!({
                "type": "server-stop",
            })
        }
    }
}
