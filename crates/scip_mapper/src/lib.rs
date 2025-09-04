use anyhow::Result;
use protocol::{EdgeIR, OccurrenceIR, Resolution, SymbolIR};
use std::collections::HashMap;

pub struct ScipMapper {
    provenance: HashMap<String, String>,
}

impl ScipMapper {
    pub fn new(indexer_name: &str, indexer_version: &str) -> Self {
        let mut provenance = HashMap::new();
        provenance.insert("source".to_string(), format!("{}@{}", indexer_name, indexer_version));
        
        Self { provenance }
    }
    
    pub fn map_scip_to_ir(
        &self,
        _scip_content: &[u8],
    ) -> Result<(Vec<SymbolIR>, Vec<EdgeIR>, Vec<OccurrenceIR>)> {
        let symbols = vec![];
        let edges = vec![];
        let occurrences = vec![];
        
        Ok((symbols, edges, occurrences))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_mapper() {
        let mapper = ScipMapper::new("scip-typescript", "1.0.0");
        assert!(mapper.provenance.contains_key("source"));
    }
}