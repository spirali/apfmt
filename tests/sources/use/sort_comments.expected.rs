use a;
use b; // about b
use c;

use x::{
    /* about a */ a,
    // about b
    b, c, // c trailing
    // closing
};

use x::{a /* y */, b, c /* x */};

use x::{ // first line
    /* b */ b, c, d, // d
};

use x::{/* leading a */ a, b, c};

use a::b; /* after */
