// own line
//!inner doc  
///outer doc
////four slashes
//
//
//	after tab
//-------
//#[derive(Debug)]
//(note) text
/// Markdown line break:  
/// next line
fn f() {
    let x = 1; // trailing
    let y = 2; //
    // let y = 2;
    g(/* a */ 1, /**/ 2, /* b */ 3, /** c*/ 4, /*!d*/ 5, /*---*/ 6);
    /* multi
       line   */
    //
}
