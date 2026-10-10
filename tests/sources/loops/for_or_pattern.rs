// A broken or-pattern ends on a `| C` line, so the head is broken and `{` goes on its own line
fn f() {
    for Some(LongVariantNameAlpha(a)) | Some(LongVariantNameBeta(a)) | Some(LongVariantNameGammaXYZ(a)) in values {
        foo(a);
    }
}
