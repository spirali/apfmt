fn overflowing() {
    let (task, code) = server
        .create_dynamic_search_rule("rule-y", json!({
            "actions": [{ "selector": { "id": "1" }, "action": { "type": "pin", "position": 0 } }]
        }))
        .await;
    let x = &[first_element, json!({
        "actions": [{ "selector": { "id": "1" }, "action": { "type": "pin", "position": 0000 } }]
    })];
    request("a", &json!({
        "actions": [{ "selector": { "id": "1" }, "action": { "type": "pin", "position": 000000 } }]
    }));
    request("b", m! {
        "actions": [{ "selector": { "id": "1" }, "action": { "type": "pin", "position": 0000000 } }]
    });
}

fn fitting() {
    request("rule-y", json!({
        "a": 1
    }));
    request("rule-y", m! {
        a b c
    });
}

fn not_hugging_in_source() {
    let (task, code) = server
        .create_dynamic_search_rule(
            "rule-y",
            json!({
                "actions": [{ "selector": { "id": "1" }, "action": { "type": "pin", "pos": 0 } }]
            }),
        )
        .await;
}

fn parsed() {
    request("rule-y", vec![
        aaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccc, ddddddd
    ]);
}
