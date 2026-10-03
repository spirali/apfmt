fn main() {
    numbers(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40);
    names(x0aaaaaaaa, x1aaaaaaaa, x2aaaaaaaa, x3aaaaaaaa, x4aaaaaaaa, x5aaaaaaaa, x6aaaaaaaa, x7aaaaaaaa, x8aaaaaaaa);
    too_wide(x0aaaaaaaaa, x1aaaaaaaaa, x2aaaaaaaaa, x3aaaaaaaaa, x4aaaaaaaaa, x5aaaaaaaaa, x6aaaaaaaaa);
    mixed_widths(aaaaaaaaaa, b, cccccccccc, d, eeeeeeeeee, f, gggggggggg, h, iiiiiiiiii, j, kkkkkkkkkk, l, mmmmmmmmmm);
    literals(true, false, 'a', 1.5, -x, !y, *z, 0xff, "s", b'c', true, false, 'a', 1.5, -x, !y, *z, 0xff);
    over_call_width(aaaaaaaa, bbbbbbbb, cccccccc, dddddddd, eeeeeeee, ffffffff, gggggggg);
    paths(a::b, c::d, e::f, g::h, i::j, k::l, m::n, o::p, q::r, s::t, u::v, w::x, y::z, a::c);
    calls(f(1), g(2), h(3), i(4), j(5), k(6), l(7), m(8), n(9), o(1), p(2), q(3), r(4), s(5));
    one_long(aaaa, bbbb, cccc, dddd, eeee, ffff, gggg, hhhh, iiii, jjjj, kkkk, llll, very_long_argument_name);
    short(1, 2, 3);
    outer(inner(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30));
}
