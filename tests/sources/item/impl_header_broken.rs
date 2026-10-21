// A header that does not fit breaks before `for`, then after the generics; the generics and the
// generic arguments stay on one line
impl<EK: KvEngine, ER: RaftEngine, T: Transport> PollHandler<PeerFsm<EK, ER>, StoreFsm<EK>> for RaftPoller<EK, ER, T> {
    fn f() {}
}
impl<Format, C1: Default + ErrorCode, C2: Default + ErrorCode> MergeWithError<DeserrError<Format, C2>> for DeserrError<Format, C1> {
    fn merge() {}
}
impl TryFrom<crate::client_server_contract::client_server_contract::PluginAlias> for crate::input::layout::PluginAlias {
    type Error = anyhow::Error;
}

// Without a trait: the type on the next line
impl<TNpmCacheHttpClient: NpmCacheHttpClient, TSys: NpmInstallerSys> NpmInstaller<TNpmCacheHttpClient, TSys> {
    fn new() {}
}

// With a `where` clause, which puts `{` on its own line
impl<Key, Storage, Edge, BreakValue, DefaultEdgeFn, PreVisitFn, PostVisitFn> NodeVisitor for FnVisitors<Key, Storage, Edge, BreakValue, DefaultEdgeFn, PreVisitFn, PostVisitFn>
where
    DefaultEdgeFn: FnMut() -> Edge,
    PreVisitFn: FnMut(Key, &mut Storage, &mut dyn NodeEdgesProvider<Edge>) -> ControlFlow<BreakValue, SubtreeVisit>,
{
    fn f() {}
}

// Neither fits: after the generics and before `for`
impl<C: OpMappingContext, R: 'static, E: JsErrorClass + 'static> FutureContextMapper<PendingOp<C>, PendingOpInfo, Result<R, E>> for PendingOpMappingInfo<C, R, true> {
    fn context(&self) {}
}
impl<Type1, Type2, Type3, Type4, Comparator1, Comparator2, Comparator3, Comparator4> Comparator<(Type1, Type2, Type3, Type4)> for (Comparator1, Comparator2, Comparator3, Comparator4)
where
    Comparator1: Comparator<Type1>,
{
    fn f() {}
}

// A type that does not fit on one line breaks on its own line
impl BatchIndexLookUpExecutor<Box<dyn Storage<Statistics = ()>>, Box<dyn BatchExecutor<StorageStats = ()>>, Box<dyn TableTaskIterBuilder<Iterator = ()>>, ApiV1> {
    fn f() {}
}
