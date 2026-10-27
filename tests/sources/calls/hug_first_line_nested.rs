// A single call argument hugs only while its first line is within `fn_call_width`; the limits of
// nested hugs add up, and the outer calls hug rather than the inner ones. `Bar(ppp(` is exactly 60
// wide: rustfmt reserves a column for each inner `)` and does not hug it.
fn f() {
    match n {
        1 => Ok(DynamicImage::ImageLuma8(process_image_buffer_from_raw_bytes(width, height, buffer, bytes_per_pixel))),
        2 => Ok(Image(process(width, height, buffer, bytes_per_pixel_xxxxxxxxxxxxxxxxxxxxxxxxxxxx))),
    }
    let x = Ok(Bar(ppppppppppppppppppppppppppppppppppppppppppppppppppppppp(width, height, buffer, bytes_per_pixel)));
    let command = ProtobufPluginCommand {
        payload: Some(Payload::ToggleFloatingPanesPayload(ToggleFloatingPanesPayload { tab_id })),
    };
    let err = Err(crate::TantivyError::AggregationError(AggregationError::InvalidRequest(format!(
        "The `{parameter}` parameter is not supported in the `top_hits` aggregation"
    ))));
}

// Not when the hugs end in a closure or a chain, whose first lines have width rules of their own
// (rustfmt limits them too)
fn g() {
    scheduler.schedule(Task::Validate(Validate::OldValueCache(Box::new(move |old_value_cache| {
        tx_.send(old_value_cache.access_count()).unwrap();
    }))));
    Ok(Some(urlencoding::decode(encoded).map_err(|inner| DecodeError::UrlDecoding {
        inner,
        header: PROXY_ORIGIN_REMOTE_HEADER,
    })?))
}
