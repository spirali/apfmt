// File comment
#![allow(dead_code)]
#![cfg_attr(  feature = "x" , allow( unused ) )]

//! Crate docs

use std::fmt;

mod m {
    //! Module docs
    #![allow(unused)]

    fn f() {
        #![allow(unused_variables)]
        let x = 1;
    }
}
