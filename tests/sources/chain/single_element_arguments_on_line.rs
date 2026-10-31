fn f() {
    {
        {
            {
                columnar_writer.record_column_type(field_entry.name(), column_type, sort_values_within_row);
                document_tokenizer.tokenize_document(current_document, &mut should_tokenize, &mut token_fn)?;
            }
        }
    }
    {
        {
            match self.pending_notify.compare_exchange_weak(n, n - 1, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => return,
                Err(x) => n = x,
            }
        }
    }
}
