// A single tuple generic argument that must break hugs `<>`
impl
    From<&(
        Option<String>,
        crate::input::layout::TiledPaneLayout,
        Vec<crate::input::layout::FloatingPaneLayout>,
    )> for TabMetadata
{
    fn f() {}
}

struct S {
    hugged: Option<(
        EvalContextGetCodeCacheCallbackFunction,
        EvalContextCodeCacheReadyCallbackFunction,
        EvalContextCodeCacheReadyCallbackFunction,
    )>,
    hugged_mut: Box<&mut (
        EvalContextGetCodeCacheCallbackFunction,
        EvalContextCodeCacheReadyCallback,
        EvalContextCodeCacheReadyCallbackFunction,
    )>,
    hugged_ptr: Box<*const (
        EvalContextGetCodeCacheCallbackFunction,
        EvalContextCodeCacheReadyCallback,
        EvalContextCodeCacheReadyCallbackFunction,
    )>,
    hugged_turbofish: [u8; size_of::<
        (EvalContextGetCodeCacheCallbackFunction, EvalContextCodeCacheReadyCallback, u8),
    >()],
    // the tuple fits on its own line: broken at the outermost scope
    outer: mpsc::UnboundedSender<
        (String, broadcast::Sender<Result<TlsKey, ErrorType>>, usize, u8, u8),
    >,
    // not a single tuple: not hugged
    two_args:
        HashMap<String, (EvalContextGetCodeCacheCallbackFunction, EvalContextCodeCacheReadyCb)>,
    slice:
        Option<&[EvalContextGetCodeCacheCallbackFunction; EVAL_CONTEXT_CODE_CACHE_READY_CALLBACKS]>,
}

fn f(
    x: Vec<(EvalContextGetCodeCacheCallbackFunction, EvalContextCodeCacheReadyCallbackFunction)>,
) {}
