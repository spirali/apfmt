// Macro arguments are copied unchanged if formatting them would change whether adjacent
// punctuation is joined, which a procedural macro sees (`Punct::spacing`)

fn copied() {
    let a = arg!(-f --foo);
    extend!($self <- fast_collect($par_iter));
    m!(x<-y);
    m!(& &x);
    m!(a  ..  .. b);
    m!(Vec< <T as Trait>::Assoc>::new());
}

// Formatting keeps the spacing of punctuation here
fn formatted() {
    m!(!!x  , &*y);
    m!(a..=-1);
    m!(x?.y);
    m!(x?
        .y);
    m!(|&x| x);
    m!(f::<Vec<Vec<u8>>>());
    m!(<T as Trait>::f());
    m!(a  ..  b);
    m!(x  as  Vec<u8>);
    // Commas and semicolons are separators
    m!(a,-1);
    m!(a  + b,c);
}
