use a::b; // trailing
// before
use a::{b, c, /* last */};

use a::{
    b, // comment b
    c, /* c */
    // own line
    d,
};

use a::{
    b,
    // dangling
};

use a::{b /* x */, c};

use a::{b::{c, // line
d}, e};
