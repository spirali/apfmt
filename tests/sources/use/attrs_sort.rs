use d;
#[cfg(y)]
use a;
#[cfg(x)]
use a;
/// Doc comments and attributes move with their item.
// plain
#[cfg(z)] use c::{y, x};
#[macro_use]
use b;
use a;

use y;
/// doc
use x;
use w;

use q;
// An own-line comment starts a new group.
/// doc
use p;

use n; #[cfg(x)] use m;
use k; /// doc of `j`
use j;

mod m {
    pub use z; // z
    /// Re-export.
    #[cfg(x)]
    pub use y::{b, a};
    use x::{};
    /// Kept with its doc comment.
    use w::{};

    fn g() {
        #[allow(unused)]
        use c;
        use b;
    }
}
