fn main() {
    if !output.status.success() {
        return Err(format!(
            "rocm-smi exited with error code {}\nStdout: {stdout}\nStderr: {stderr}",
            output.status
        ).into());
    }
    foo(bar(
        first_argument_value_xxxxxxxx,
        second_argument_value_xxxxxxxxxx,
    ).await?);
    process(&compute(
        first_argument_value_xxxxxxxxxxxx,
        second_argument_value_xxxxxxxxxxxxxx,
    ).unwrap());
    // two elements: not joined
    outer(
        inner(
            first_argument_value_xxxxxxxx,
            second_argument_value_xxxxxxxxxxxx,
        )
        .first()
        .unwrap(),
    );
    Err(anyhow::anyhow!("Cannot forget job {job_id}. It either hasn't been completed yet").into())
}
