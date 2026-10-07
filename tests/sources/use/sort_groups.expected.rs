use c;
use d;

use a;
use b;
// A comment line starts a new group
use y;
use z;

use self::z;
use super::y;
use crate::x;
use ::w;
use anyhow::Result;
pub use b::c;
use std::io;

use a;
use a as b;
use a as c;
use a::b;
use a::*;
use a::{c, d};

use _a;
use U8;
use a2;
use a10;
use r#struct;
use r#type;
use u8;

use k;
use l;

fn main() {
    use c;
    use d;
    let x = 1;
    use a;
    use b;
}
