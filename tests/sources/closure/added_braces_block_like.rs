// Braces added around a block-like body cost a split: the body stays without them, and the chain
// breaks instead (as rustfmt)
fn f() {
    if a {
        let indices = groups
            .iter_mut()
            .enumerate()
            .flat_map(|(group_id, group)| {
                std::mem::take(group).into_iter().map(move |index| AllocationIndex { index, group_idx: group_id as u32, fractions: 0 })
            })
            .collect();
    }
}
