use a::{b};

use a::{b as c};

use a::{b::{c}};

use a::{b::{c, d}};

use a::{self};

use {b};

use a::{c, d::{}};

use a::{};

use a::{b::{c}, d};

use e::{x, b::{}};
fn f() {
    use a::{};
}
