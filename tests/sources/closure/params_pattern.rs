fn f() {
    let h = map(|HybridQuery { semantic_ratio, embedder }| x);
    let h = |HybridQuery { semantic_ratio, embedder }| x;
    let h = |HybridQuery { semantic_ratio, embedder }| { x; y };
    let h = |&HybridQuery { semantic_ratio, embedder }| x;
    let h = |a: u8, HybridQuery { semantic_ratio, embedder }: H| x;
    let h = |(name, Remote { url, search_api_key, write_api_key })| x;
    let h = |[name, Remote { url, search_api_key, write_api_key }]| x;
    let h = |Foo(name, Remote { url, search_api_key, write_api_key })| x;
    let h = |&(name, Remote { url, search_api_key, write_api_key })| x;
    let h = |(name, Remote { url, search_api_key, write_api_key }): (A, B)| x;
    let h = |(nameeeeeeeeeeeeeeeeeeeeeeeeeeee, urlllllllllllllllllllllll, search_api_keyyyyyyyyyyyyyyy)| x;
    let h = |(a, b)| a + b;
    items.map(|(index, item)| (index, item.value));
}
