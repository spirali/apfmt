use d;
use c;

use b;
use a;
// A comment line starts a new group
use z;
use y;

use std::io;
use crate::x;
use super::y;
use self::z;
use ::w;
use anyhow::Result;
pub use b::c;

use a::b;
use a as c;
use a as b;
use a;
use a::*;
use a::{c, d};

use u8;
use U8;
use _a;
use a10;
use a2;
use r#type;
use r#struct;

use l; use k;

fn main() {
    use d;
    use c;
    let x = 1;
    use b;
    use a;
}
