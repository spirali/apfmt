// A root broken over more lines followed by a single element
async fn call_root() {
    let response = command_job_output(gsettings, &mut session, opts.job_selector, get_task_selector(Some(opts.task_selector)), opts.stream).await?;
    let filler = std::iter::repeat_n(style.unused_area, num_characters as usize - indicator_count as usize).collect::<String>();
}

fn block_root() {
    let run_time = match task_info.end_time {
        None => current_time.duration_since(task_info.start_time),
        Some(end_time) => end_time.duration_since(task_info.start_time),
    }.unwrap_or_default();
    FilledRectangle { x: 0.0, y: value, width: 10.0, color: self.color }.draw(painter);
}

// The element does not fit after `)`: on its own line at the indentation of the root
fn long_element() {
    foo(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb).quxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx();
}

// A root short on a single line that does not fit: the single element still follows `)`
async fn short_root() {
    if c0 {
        if c1 {
            if c2 {
                if c3 {
                    if c4 {
                        if c5 {
                            if c6 {
                                if c7 {
                                    if c8 {
                                        if c9 {
                                            if c10 {
                                                some_function_name(argument_one, argument_two, argument_three).await?;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
