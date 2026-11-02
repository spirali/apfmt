fn f() {
    {
        {
            {
                {
                    {
                        {
                            match &local_branch.details {
                                BranchDetails::Local(details) => details.upstream.as_ref().is_some_and(|upstream| {
                                    upstream.reference == remote_branch.reference
                                }),
                                BranchDetails::Remote(_) => false,
                            };
                        }
                    }
                }
            }
        }
    }
}
