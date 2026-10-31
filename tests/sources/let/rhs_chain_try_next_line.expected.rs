fn f() {
    {
        {
            let embedder_id =
                find_free_index().ok_or(crate::UserError::TooManyEmbedders(total_embedder_count))?;
        }
        let block_postings =
            inverted_index.read_block_postings_from_terminfo(&term_info, IndexRecordOption::Basic)?;
        let predefined_acl =
            parse_predefined_acl(&input.predefined_acl).or_invalid_input("invalid predefined_acl")?;
    }
}
