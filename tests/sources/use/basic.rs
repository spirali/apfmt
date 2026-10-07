use   std :: io ;

pub ( crate ) use a::b as c;

use a::b as _;

use a :: * ;

use ::std::fmt;

use a::{b, c, d,};

use a::{self, b};

use a::{b as c, d};

use {a, b};

use ::{a, b};

pub(super) use self::x::{w, y::z};
fn main() {}
