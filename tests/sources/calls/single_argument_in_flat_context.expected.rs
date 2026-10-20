fn main() {
    let schema = schema.or_else(|| {
        Some(columns_names_to_empty_schema(
            names.iter().map(String::as_str),
        ))
    });
    let repo_path = match absolute_path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return Err(
                LabeledError::new(
                    format!("error canonicalizing [{}]", absolute_path.display()),
                ).with_label(e.to_string(), path.span),
            );
        }
    };
}
