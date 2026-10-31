fn f() {
    {
        let quoted = (contents.len() >= 3
            && contents.starts_with(b"$\"")
            && contents.ends_with(b"\""))
            || is_quoted(contents);
        {
            let (key_uuid, search_rules) = match ActionPolicy::<A>::authenticate_tenant_token(
                &auth, token,
            ) {
                Ok(TenantTokenOutcome::Valid(key_uuid, search_rules)) => {
                    (key_uuid, Some(search_rules))
                }
                Ok(TenantTokenOutcome::NotATenantToken) | Err(AuthError::InvalidTenantToken) => (
                    auth.get_optional_uid_from_encoded_key(token.as_bytes())
                        .map_err(|_e| AuthError::InvalidApiKey)?
                        .ok_or(AuthError::InvalidApiKey)?,
                    None,
                ),
                Err(e) => return Err(e),
            };
        }
    }
}
