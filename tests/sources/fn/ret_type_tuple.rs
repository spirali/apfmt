// A tuple in a return type stays on one line where it fits: after broken parameters, or on the next
// line without parameters (rustfmt breaks the tuple)
impl X {
    fn render(&mut self, client_id: Option<ClientId>) -> Result<Option<(Vec<CharacterChunk>, Option<String>, Vec<SixelImageChunk>, Vec<KittyImageChunk>)>, ZellijError> {
        1
    }
    fn write_engine_call(&self, call: EngineCall<PipelineData>) -> Result<(PipelineDataWriter<Self>, mpsc::Receiver<EngineCallResponse<PipelineData>>), ShellError> {
        1
    }
}

pub fn must_new_cluster_and_kv_client() -> (Cluster<ServerCluster<RocksEngine>, RocksEngine>, TikvClient, Context) {
    must_new_cluster_and_kv_client_mul(1)
}
