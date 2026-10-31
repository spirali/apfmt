fn saves_lines() {
    {
        let Some((parent_static_id, file_name)) = self.tests.get(&desc.parent_id).and_then(|parent_desc| {
            let parent_static_id = parent_desc.static_id().to_string();
            Some((parent_static_id, root_desc.location().file_name.clone()))
        }) else {
            return;
        };
        let Some((specifier, media_type, _)) = state.state_snapshot.document_modules.resolve_dependency(
            &specifier,
            referrer,
            ResolutionMode::Import,
            scope.as_deref(),
        ) else {
            continue;
        };
    }
}

fn saves_no_lines() {
    let Some(x) = self.entries.lock().ok().and_then(|guard| guard.get(key).cloned()) else {
        return;
    };
    let Some(parent) = self.tests.get(&desc.parent_id).and_then(|parent_desc| {
        let parent_static_id = parent_desc.static_id().to_string();
        Some(parent_static_id)
    }) else {
        return;
    };
}
