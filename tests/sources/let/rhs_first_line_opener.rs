fn f() {
    {
        let specifier_len = u32::from_le_bytes(buffer[specifier_end_pos..source_hash_start_pos].try_into()?) as usize;
        req.data = self.handle_data.request_callbacks.borrow_mut().insert(RequestCallbackState::Write(WriteRequestCallbackState {
            isolate: unsafe { scope.as_raw_isolate_ptr() },
            req_wrap_obj: v8::Global::new(scope, req_wrap_obj),
            stream_handle,
        }));
    }
}
