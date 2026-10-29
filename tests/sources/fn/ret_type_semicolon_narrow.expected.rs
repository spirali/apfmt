trait T {
  fn workspace_external_import_map_loade(
    &self,
  ) -> Result<
    &WorkspaceExternalImportMapLoaderRc<TSys>,
    WorkspaceDiscoverError,
  >;
}
