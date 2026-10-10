// A broken or-pattern and an iterable that does not fit after it: broken before `in`
fn f() {
    for Some(LongVariantNameAlpha(a)) | Some(LongVariantNameBeta(a)) | Some(LongVariantNameGammaXYZ(a)) in compute(argument_one, argument_two, argument_three, argument_four) {
        foo(a);
    }
}
