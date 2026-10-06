fn main() {
    f(/* inline */);
    f( // line
    );
    f(
        // own line
    );
    if x { /* inline */ }
    if x { // line
    }
    if x {
        // own line
    }
}
fn p(/* inline */) {}
fn q( // line
) {}
fn b1() { /* inline */ }
fn b2() { // line
}
fn b3() {
    // own line
}
struct T1(/* inline */);
struct T2( // line
);
struct S1 { /* inline */ }
struct S2 { // line
}
struct S3 {
    // own line
}
enum E1 { /* inline */ }
enum E2 { // line
}
