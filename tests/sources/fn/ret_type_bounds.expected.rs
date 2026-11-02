impl X {
    // The line does not fit: broken before every `+`
    pub fn lines(
        &self,
        aaaaaaaaaaaa: Text<'a>,
    ) -> impl DoubleEndedIterator<Item = RopeSlice<'a>>
        + ExactSizeIterator<Item = RopeSlice<'a>>
        + 'a {
        1
    }
    // Only ` {` does not fit: on its own line
    pub fn brace(
        &self,
        aaaaaaaaaaaa: Text<'a>,
    ) -> impl DoubleEndedIterator<Item = RopeSlice<'a>> + ExactSizeIterator<Item = RopeSl<'a>> + 'a
    {
        1
    }
    // Without parameters to break: block-indented
    fn no_params() -> impl DoubleEndedIterator<Item = RopeSliceeeeeee<'a>>
        + ExactSizeIterator<Item = RopeSliceeeeee<'a>>
        + 'a {
        1
    }
    fn no_params_wider() -> impl DoubleEndedIterator<Item = RopeSliceeeeeee<'a>>
        + ExactSizeIterator<Item = RopeSliceeeeeeeeeeeeeeeeeeeeeeeeeee<'a>>
        + 'a {
        1
    }
}
