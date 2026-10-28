// A tab-indented source is re-indented with spaces also where afmt copies text: the continuation
// lines of a block comment keep their alignment (rustfmt moves them to the indentation of the code)
// and a copied macro body keeps its nesting, a tab counting as `tab_spaces` columns
fn main() {
	if x {
		/* block
		   comment */
		let y = m! {
			a b
				c
		};
		let s = "a	b";
		f(); /* after
		        code */
	}
}
