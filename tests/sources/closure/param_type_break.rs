// A single parameter with a long type breaks the generic arguments of the type between the pipes
fn f() {
    {
        {
            storage.sched_txn_command(
                cmd,
                Box::new(move |res: storage::Result<std::result::Result<PessimisticLockResults, StorageError>>| done_tx.send(res).unwrap()),
            );
            let f = move |res: storage::Result<std::result::Result<PessimisticLockResults, StorageError>>| {
                done_tx.send(res).unwrap();
                other();
            };
            let h = |res: Result<Vec<u8>, String>| res.unwrap_or_else(|_| Vec::new()).into_iter().count();
        }
    }
}
