// The generic arguments of a broken bound are block-indented, with `>` at the indentation of the
// bound; rustfmt indents them one level deeper, in `where`, in generics and after `dyn`
struct S<D>(D)
where
    D: Dispatch<PollItem = MessageHead<T::Outgoing>, PollBody = Bssssssss, RecvItem = MessageHead<T::Incoming>> + Unpin;

fn g<D>(d: D)
where
    D: Dispatch<PollItem = MessageHead<T::Outgoing>, PollBody = Bssssssss, RecvItem = MessageHead<T::Incoming>> + Unpin,
    E: Dispatch<PollItem = MessageHead<T::Outgoing>, PollBody = Bssssssss, RecvItem = MessageHead<T::Incoming>>,
{
    1
}

fn h<D: Dispatch<PollItem = MessageHead<T::Outgoing>, PollBody = Bssssssss, RecvItem = MessageHead<T::Incoming>> + Unpin>(d: D) {
    1
}

type T = Box<dyn Dispatch<PollItem = MessageHead<T::Outgoing>, PollBody = Bssssssss, RecvItem = MessageHead<T::Incoming>> + Unpin>;
