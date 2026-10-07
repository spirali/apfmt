use c;
use b; // about b
use a;

use x::{
    c, // c trailing
    // about b
    b,
    /* about a */ a,
    // closing
};

use x::{c /* x */, b, a /* y */};

use x::{ // first line
    d, // d
    c,
    /* b */ b,
};

use x::{c, b, /* leading a */ a};

use a::{b} /* after */;
