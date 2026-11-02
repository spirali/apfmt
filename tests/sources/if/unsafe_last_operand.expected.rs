fn f() {
    {
        {
            {
                {
                    let (b, count) = if count == 0
                        && unsafe {
                            std::ptr::eq(b.as_ptr().add(b.len()), bytes.as_ptr().add(bytes.len()))
                        }
                    {
                        1
                    } else {
                        2
                    };
                }
            }
        }
    }
}
