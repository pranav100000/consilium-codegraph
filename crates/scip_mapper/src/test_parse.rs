#[cfg(test)]
mod tests {
    use crate::{ScipOccurrence, ScipIndex, ScipMapper};
    use serde_json;

    #[test]
    fn test_full_scip_index_parsing() {
        // Test JSON parsing directly since we don't have scip CLI
        let json_data = std::fs::read_to_string("/Users/pranavsharan/Developer/consilium-codegraph/test_scip_index.json")
            .expect("Could not read test SCIP index");

        let result: Result<crate::ScipIndex, _> = serde_json::from_str(&json_data);
        match result {
            Ok(scip_index) => {
                println!("✅ Successfully parsed full SCIP index");
                println!("  Tool: {} {}", scip_index.metadata.tool_info.name, scip_index.metadata.tool_info.version);
                println!("  Documents: {}", scip_index.documents.len());
                for (i, doc) in scip_index.documents.iter().enumerate() {
                    println!("    {}: {} ({} symbols, {} occurrences)", 
                        i, doc.relative_path, doc.symbols.len(), doc.occurrences.len());
                }
            },
            Err(e) => {
                println!("❌ Failed to parse full SCIP index: {}", e);
                panic!("Full SCIP parsing failed: {}", e);
            }
        }
    }

    #[test]
    fn test_scip_full_document_parsing() {
        let json_data = std::fs::read_to_string("/tmp/test_document.json")
            .expect("Could not read test document");

        let result: Result<crate::ScipDocument, _> = serde_json::from_str(&json_data);
        match result {
            Ok(doc) => {
                println!("✅ Successfully parsed document: {}", doc.relative_path);
                println!("  Symbols: {}", doc.symbols.len());
                println!("  Occurrences: {}", doc.occurrences.len());
            },
            Err(e) => {
                println!("❌ Failed to parse document: {}", e);
                panic!("Document parsing failed: {}", e);
            }
        }
    }

    #[test]  
    fn test_scip_occurrence_parsing() {
        let json_data = r#"[
          {
            "range": [0, 0, 0],
            "symbol": "scip-typescript npm . . `user.ts`/",
            "symbol_roles": 1,
            "enclosing_range": [0, 0, 20, 1]
          },
          {
            "range": [1, 2, 4],
            "symbol": "scip-typescript npm . . `user.ts`/User#id.",
            "symbol_roles": 1
          }
        ]"#;

        let result: Result<Vec<ScipOccurrence>, _> = serde_json::from_str(json_data);
        match result {
            Ok(occurrences) => {
                println!("✅ Successfully parsed {} occurrences", occurrences.len());
                for (i, occ) in occurrences.iter().enumerate() {
                    println!("  {}: {} (roles: {:?})", i, occ.symbol, occ.symbol_roles);
                }
            },
            Err(e) => {
                println!("❌ Failed to parse: {}", e);
                panic!("Parsing failed: {}", e);
            }
        }
    }
}