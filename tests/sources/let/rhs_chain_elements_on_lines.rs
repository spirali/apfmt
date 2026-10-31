fn f() {
    {
        {
            let items = fields.unnamed.iter().zip(accessor).map(|(_, accessor)| quote!(nu_protocol::IntoValue::into_value(#accessor, span)));
            {
                {
                    {
                        {
                            _ = sender.sender.take().unwrap().send(exception_to_err_result(tc_scope, exception, true, false));
                        }
                    }
                }
            }
        }
    }
}
