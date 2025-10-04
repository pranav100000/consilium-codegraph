use anyhow::Result;
use scip_mapper::{ScipIndex, ScipDocument, ScipSymbol, ScipOccurrence, ScipMetadata, ScipToolInfo, ScipRelationship};
use std::fs;
use tempfile::TempDir;
use protocol::{Language, SymbolKind, OccurrenceRole, EdgeType, Resolution};

/// Comprehensive tests for SCIP parsing and IR mapping
/// These tests ensure SCIP files are correctly parsed and mapped to internal IR

#[test] 
fn test_scip_index_parsing() -> Result<()> {
    println!("🔍 Testing SCIP index parsing...");
    
    let temp_dir = TempDir::new()?;
    
    // Create a mock SCIP index JSON
    let scip_json = r#"{
  "metadata": {
    "tool_info": {
      "name": "scip-typescript",
      "version": "0.3.33"
    },
    "project_root": "file:///test/project",
    "text_document_encoding": 0
  },
  "documents": [
    {
      "relative_path": "src/user.ts",
      "symbols": [
        {
          "symbol": "scip-typescript npm . . `src/user.ts`/User#",
          "documentation": ["User interface for the application"],
          "relationships": []
        },
        {
          "symbol": "scip-typescript npm . . `src/user.ts`/UserService#",
          "documentation": ["Service class for managing users"],
          "relationships": [
            {
              "symbol": "scip-typescript npm . . `src/user.ts`/User#",
              "isReference": true
            }
          ]
        },
        {
          "symbol": "scip-typescript npm . . `src/user.ts`/UserService#getUser().",
          "documentation": ["Get a user by ID"],
          "relationships": [
            {
              "symbol": "scip-typescript npm . . `src/user.ts`/User#",
              "isReference": true
            }
          ]
        }
      ],
      "occurrences": [
        {
          "range": [0, 17, 21],
          "symbol": "scip-typescript npm . . `src/user.ts`/User#",
          "symbol_roles": 1
        },
        {
          "range": [5, 13, 24],
          "symbol": "scip-typescript npm . . `src/user.ts`/UserService#",
          "symbol_roles": 1
        },
        {
          "range": [7, 4, 11],
          "symbol": "scip-typescript npm . . `src/user.ts`/UserService#getUser().",
          "symbol_roles": 1
        },
        {
          "range": [7, 20, 24],
          "symbol": "scip-typescript npm . . `src/user.ts`/User#",
          "symbol_roles": 2
        }
      ]
    }
  ]
}"#;
    
    // Write the SCIP file
    let scip_file = temp_dir.path().join("index.scip.json");
    fs::write(&scip_file, scip_json)?;
    
    // Parse using serde_json directly (simulating ScipMapper parsing)
    let parsed_index: ScipIndex = serde_json::from_str(scip_json)?;
    
    // Verify metadata
    assert_eq!(parsed_index.metadata.tool_info.name, "scip-typescript");
    assert_eq!(parsed_index.metadata.tool_info.version, "0.3.33");
    assert_eq!(parsed_index.metadata.project_root, "file:///test/project");
    
    // Verify documents
    assert_eq!(parsed_index.documents.len(), 1);
    let doc = &parsed_index.documents[0];
    assert_eq!(doc.relative_path, "src/user.ts");
    
    // Verify symbols
    assert_eq!(doc.symbols.len(), 3);
    let user_symbol = &doc.symbols[0];
    assert_eq!(user_symbol.symbol, "scip-typescript npm . . `src/user.ts`/User#");
    assert_eq!(user_symbol.documentation.as_ref().unwrap()[0], "User interface for the application");
    
    let service_symbol = &doc.symbols[1];
    assert_eq!(service_symbol.symbol, "scip-typescript npm . . `src/user.ts`/UserService#");
    assert!(service_symbol.relationships.as_ref().unwrap().len() > 0);
    
    // Verify occurrences
    assert_eq!(doc.occurrences.len(), 4);
    let first_occurrence = &doc.occurrences[0];
    assert_eq!(first_occurrence.range, vec![0, 17, 21]);
    assert_eq!(first_occurrence.symbol_roles, Some(1)); // Definition
    
    println!("✅ SCIP index parsing successful");
    Ok(())
}

#[test]
fn test_ir_mapping_symbols() -> Result<()> {
    println!("🔄 Testing IR mapping for symbols...");
    
    let scip_mapper = scip_mapper::ScipMapper::new("test-indexer", "1.0.0");
    
    // Create a simple SCIP symbol
    let scip_symbol = ScipSymbol {
        symbol: "scip-typescript npm . . `src/user.ts`/User#".to_string(),
        documentation: Some(vec!["A user class".to_string()]),
        relationships: Some(vec![
            ScipRelationship {
                symbol: "scip-typescript npm . . `src/base.ts`/BaseEntity#".to_string(),
                is_implementation: Some(false),
                is_reference: Some(true),
            }
        ]),
    };
    
    // Create a SCIP index with this symbol
    let scip_index = ScipIndex {
        metadata: ScipMetadata {
            tool_info: ScipToolInfo {
                name: "scip-typescript".to_string(),
                version: "0.3.33".to_string(),
            },
            project_root: "file:///test".to_string(),
            text_document_encoding: Some(0),
        },
        documents: vec![
            ScipDocument {
                relative_path: "src/user.ts".to_string(),
                symbols: vec![scip_symbol],
                occurrences: vec![],
            }
        ],
    };
    
    // Map to IR
    let commit_sha = "abc123";
    let (symbols, edges, _occurrences) = scip_mapper.map_scip_to_ir(&scip_index, commit_sha)?;
    
    // Verify symbol mapping
    assert_eq!(symbols.len(), 1);
    let symbol = &symbols[0];
    assert_eq!(symbol.lang, Language::TypeScript);
    assert!(symbol.name.contains("User"));
    assert!(symbol.fqn.contains("User"));
    assert_eq!(symbol.kind, SymbolKind::Class); // Should detect # as class
    assert_eq!(symbol.doc, Some("A user class".to_string()));
    
    // Verify edge mapping (from relationships)
    assert_eq!(edges.len(), 1);
    let edge = &edges[0];
    assert_eq!(edge.edge_type, EdgeType::Calls); // Non-implementation relationship
    assert_eq!(edge.resolution, Resolution::Semantic);
    
    println!("✅ IR mapping for symbols successful");
    Ok(())
}

#[test]
fn test_ir_mapping_occurrences() -> Result<()> {
    println!("📍 Testing IR mapping for occurrences...");
    
    let scip_mapper = scip_mapper::ScipMapper::new("test-indexer", "1.0.0");
    
    // Create SCIP occurrences with different roles
    let occurrences = vec![
        ScipOccurrence {
            range: vec![10, 5, 9], // Same line: start_line, start_col, end_col
            symbol: "test_symbol".to_string(),
            symbol_roles: Some(1), // Definition
            enclosing_range: None,
        },
        ScipOccurrence {
            range: vec![15, 8, 12], // Same line reference
            symbol: "test_symbol".to_string(),
            symbol_roles: Some(2), // Reference
            enclosing_range: None,
        },
        ScipOccurrence {
            range: vec![20, 3, 22, 7], // Multi-line: start_line, start_col, end_line, end_col
            symbol: "test_symbol".to_string(),
            symbol_roles: Some(4), // Write
            enclosing_range: None,
        },
    ];
    
    let scip_index = ScipIndex {
        metadata: ScipMetadata {
            tool_info: ScipToolInfo {
                name: "scip-typescript".to_string(),
                version: "0.3.33".to_string(),
            },
            project_root: "file:///test".to_string(),
            text_document_encoding: Some(0),
        },
        documents: vec![
            ScipDocument {
                relative_path: "src/test.ts".to_string(),
                symbols: vec![],
                occurrences,
            }
        ],
    };
    
    // Map to IR
    let commit_sha = "def456";
    let (_symbols, _edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, commit_sha)?;
    
    // Verify occurrence mapping
    assert_eq!(occurrences.len(), 3);
    
    // Check definition occurrence
    let def_occ = &occurrences[0];
    assert_eq!(def_occ.role, OccurrenceRole::Definition);
    assert_eq!(def_occ.span.start_line, 10);
    assert_eq!(def_occ.span.start_col, 5);
    assert_eq!(def_occ.span.end_line, 10); // Same line
    assert_eq!(def_occ.span.end_col, 9);
    
    // Check reference occurrence
    let ref_occ = &occurrences[1];
    assert_eq!(ref_occ.role, OccurrenceRole::Reference);
    assert_eq!(ref_occ.span.start_line, 15);
    assert_eq!(ref_occ.span.start_col, 8);
    assert_eq!(ref_occ.span.end_col, 12);
    
    // Check write occurrence (multi-line)
    let write_occ = &occurrences[2];
    assert_eq!(write_occ.role, OccurrenceRole::Write);
    assert_eq!(write_occ.span.start_line, 20);
    assert_eq!(write_occ.span.start_col, 3);
    assert_eq!(write_occ.span.end_line, 22); // Different line
    assert_eq!(write_occ.span.end_col, 7);
    
    println!("✅ IR mapping for occurrences successful");
    Ok(())
}

#[test]
fn test_symbol_kind_detection() -> Result<()> {
    println!("🏷️ Testing symbol kind detection...");
    
    let scip_mapper = scip_mapper::ScipMapper::new("test-indexer", "1.0.0");
    
    let test_cases = vec![
        ("scip-typescript npm . . `src/test.ts`/MyClass#", SymbolKind::Class),
        ("scip-typescript npm . . `src/test.ts`/myFunction().", SymbolKind::Function),
        ("scip-typescript npm . . `src/test.ts`/MyClass#method().", SymbolKind::Method),
        ("scip-typescript npm . . `src/test.ts`/myVariable.", SymbolKind::Variable),
        ("scip-typescript npm . . `src/test.ts`/CONSTANT.", SymbolKind::Variable),
    ];
    
    for (symbol_string, expected_kind) in test_cases {
        let scip_symbol = ScipSymbol {
            symbol: symbol_string.to_string(),
            documentation: None,
            relationships: None,
        };
        
        let scip_index = ScipIndex {
            metadata: ScipMetadata {
                tool_info: ScipToolInfo {
                    name: "scip-typescript".to_string(),
                    version: "0.3.33".to_string(),
                },
                project_root: "file:///test".to_string(),
                text_document_encoding: Some(0),
            },
            documents: vec![
                ScipDocument {
                    relative_path: "src/test.ts".to_string(),
                    symbols: vec![scip_symbol],
                    occurrences: vec![],
                }
            ],
        };
        
        let (symbols, _edges, _occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "test123")?;
        
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].kind, expected_kind, 
            "Symbol '{}' should be detected as {:?}, got {:?}", 
            symbol_string, expected_kind, symbols[0].kind);
    }
    
    println!("✅ Symbol kind detection successful");
    Ok(())
}

#[test]
fn test_malformed_scip_handling() -> Result<()> {
    println!("⚠️ Testing malformed SCIP handling...");
    
    let scip_mapper = scip_mapper::ScipMapper::new("test-indexer", "1.0.0");
    
    // Test with malformed symbol strings
    let malformed_symbols = vec![
        ScipSymbol {
            symbol: "incomplete symbol".to_string(), // Not enough parts
            documentation: None,
            relationships: None,
        },
        ScipSymbol {
            symbol: "".to_string(), // Empty symbol
            documentation: None,
            relationships: None,
        },
        ScipSymbol {
            symbol: "a b c".to_string(), // Too few parts
            documentation: None,
            relationships: None,
        },
    ];
    
    let scip_index = ScipIndex {
        metadata: ScipMetadata {
            tool_info: ScipToolInfo {
                name: "scip-typescript".to_string(),
                version: "0.3.33".to_string(),
            },
            project_root: "file:///test".to_string(),
            text_document_encoding: Some(0),
        },
        documents: vec![
            ScipDocument {
                relative_path: "src/test.ts".to_string(),
                symbols: malformed_symbols,
                occurrences: vec![
                    // Malformed occurrence with invalid range
                    ScipOccurrence {
                        range: vec![1], // Invalid range (too few elements)
                        symbol: "test".to_string(),
                        symbol_roles: Some(1),
                        enclosing_range: None,
                    },
                    ScipOccurrence {
                        range: vec![], // Empty range
                        symbol: "test".to_string(),
                        symbol_roles: Some(2),
                        enclosing_range: None,
                    },
                ],
            }
        ],
    };
    
    // Should not panic and should handle malformed data gracefully
    let result = scip_mapper.map_scip_to_ir(&scip_index, "test123");
    assert!(result.is_ok(), "Should handle malformed SCIP data gracefully");
    
    let (symbols, _edges, occurrences) = result?;
    
    // Should filter out malformed symbols and occurrences
    // Exact behavior depends on implementation - either filter them out or convert with defaults
    println!("Processed {} symbols and {} occurrences from malformed input", 
             symbols.len(), occurrences.len());
    
    println!("✅ Malformed SCIP handling successful");
    Ok(())
}

#[test]
fn test_cross_file_relationships() -> Result<()> {
    println!("🔗 Testing cross-file relationships...");
    
    let scip_mapper = scip_mapper::ScipMapper::new("test-indexer", "1.0.0");
    
    // Create symbols with cross-file relationships
    let scip_index = ScipIndex {
        metadata: ScipMetadata {
            tool_info: ScipToolInfo {
                name: "scip-typescript".to_string(),
                version: "0.3.33".to_string(),
            },
            project_root: "file:///test".to_string(),
            text_document_encoding: Some(0),
        },
        documents: vec![
            ScipDocument {
                relative_path: "src/user.ts".to_string(),
                symbols: vec![
                    ScipSymbol {
                        symbol: "scip-typescript npm . . `src/user.ts`/User#".to_string(),
                        documentation: Some(vec!["User class".to_string()]),
                        relationships: Some(vec![
                            ScipRelationship {
                                symbol: "scip-typescript npm . . `src/base.ts`/BaseEntity#".to_string(),
                                is_implementation: Some(true), // Implements BaseEntity
                                is_reference: Some(false),
                            }
                        ]),
                    }
                ],
                occurrences: vec![],
            },
            ScipDocument {
                relative_path: "src/base.ts".to_string(),
                symbols: vec![
                    ScipSymbol {
                        symbol: "scip-typescript npm . . `src/base.ts`/BaseEntity#".to_string(),
                        documentation: Some(vec!["Base entity class".to_string()]),
                        relationships: None,
                    }
                ],
                occurrences: vec![],
            },
            ScipDocument {
                relative_path: "src/service.ts".to_string(),
                symbols: vec![
                    ScipSymbol {
                        symbol: "scip-typescript npm . . `src/service.ts`/UserService#".to_string(),
                        documentation: Some(vec!["User service".to_string()]),
                        relationships: Some(vec![
                            ScipRelationship {
                                symbol: "scip-typescript npm . . `src/user.ts`/User#".to_string(),
                                is_implementation: Some(false),
                                is_reference: Some(true), // References User
                            }
                        ]),
                    }
                ],
                occurrences: vec![],
            },
        ],
    };
    
    let (symbols, edges, _occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "test456")?;
    
    // Should have 3 symbols (User, BaseEntity, UserService)
    assert_eq!(symbols.len(), 3);
    
    // Should have 2 edges (User implements BaseEntity, UserService references User)
    assert_eq!(edges.len(), 2);
    
    // Check implementation edge
    let impl_edge = edges.iter().find(|e| e.edge_type == EdgeType::Implements);
    assert!(impl_edge.is_some(), "Should have an implementation edge");
    
    // Check reference edge  
    let ref_edge = edges.iter().find(|e| e.edge_type == EdgeType::Calls);
    assert!(ref_edge.is_some(), "Should have a reference edge");
    
    // All edges should be semantic resolution
    for edge in &edges {
        assert_eq!(edge.resolution, Resolution::Semantic);
    }
    
    println!("✅ Cross-file relationships successful");
    Ok(())
}

#[test]
fn test_large_scip_index_processing() -> Result<()> {
    println!("📊 Testing large SCIP index processing...");
    
    let scip_mapper = scip_mapper::ScipMapper::new("test-indexer", "1.0.0");
    
    // Create a large SCIP index with many symbols and relationships
    let mut documents = Vec::new();
    
    for file_idx in 0..10 {
        let mut symbols = Vec::new();
        let mut occurrences = Vec::new();
        
        for symbol_idx in 0..100 {
            let symbol_name = format!("Class{}_{}", file_idx, symbol_idx);
            symbols.push(ScipSymbol {
                symbol: format!("scip-typescript npm . . `src/file{}.ts`/{}#", file_idx, symbol_name),
                documentation: Some(vec![format!("Documentation for {}", symbol_name)]),
                relationships: if symbol_idx > 0 {
                    Some(vec![
                        ScipRelationship {
                            symbol: format!("scip-typescript npm . . `src/file{}.ts`/Class{}_{}#", 
                                          file_idx, file_idx, symbol_idx - 1),
                            is_implementation: Some(false),
                            is_reference: Some(true),
                        }
                    ])
                } else {
                    None
                },
            });
            
            // Add occurrences for each symbol
            for occ_idx in 0..5 {
                occurrences.push(ScipOccurrence {
                    range: vec![symbol_idx as i32 + occ_idx, 0, 10],
                    symbol: format!("scip-typescript npm . . `src/file{}.ts`/{}#", file_idx, symbol_name),
                    symbol_roles: Some((occ_idx % 3) + 1), // Cycle through role types
                    enclosing_range: None,
                });
            }
        }
        
        documents.push(ScipDocument {
            relative_path: format!("src/file{}.ts", file_idx),
            symbols,
            occurrences,
        });
    }
    
    let scip_index = ScipIndex {
        metadata: ScipMetadata {
            tool_info: ScipToolInfo {
                name: "scip-typescript".to_string(),
                version: "0.3.33".to_string(),
            },
            project_root: "file:///test".to_string(),
            text_document_encoding: Some(0),
        },
        documents,
    };
    
    let start_time = std::time::Instant::now();
    let (symbols, edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "large_test")?;
    let duration = start_time.elapsed();
    
    // Verify we processed everything
    assert_eq!(symbols.len(), 1000); // 10 files * 100 symbols
    assert_eq!(occurrences.len(), 5000); // 10 files * 100 symbols * 5 occurrences
    // Edges: 99 relationships per file (each symbol except first references previous)
    assert_eq!(edges.len(), 990); // 10 files * 99 edges per file
    
    // Should complete reasonably quickly
    assert!(duration.as_secs() < 5, "Large SCIP processing took too long: {:?}", duration);
    
    println!("✅ Large SCIP index processing completed in {:?}", duration);
    println!("   Processed {} symbols, {} edges, {} occurrences", 
             symbols.len(), edges.len(), occurrences.len());
    
    Ok(())
}

#[test]
fn test_multi_language_scip_processing() -> Result<()> {
    println!("🌐 Testing multi-language SCIP processing...");
    
    // Note: This test simulates different language SCIP formats
    // In practice, each language indexer produces slightly different SCIP formats
    
    let scip_mapper = scip_mapper::ScipMapper::new("multi-lang", "1.0.0");
    
    // Simulate Python SCIP symbols (different naming conventions)
    let python_scip = ScipIndex {
        metadata: ScipMetadata {
            tool_info: ScipToolInfo {
                name: "scip-python".to_string(),
                version: "0.2.0".to_string(),
            },
            project_root: "file:///python_test".to_string(),
            text_document_encoding: Some(0),
        },
        documents: vec![
            ScipDocument {
                relative_path: "user.py".to_string(),
                symbols: vec![
                    ScipSymbol {
                        symbol: "scip-python local . . user.py/User#".to_string(),
                        documentation: Some(vec!["Python User class".to_string()]),
                        relationships: None,
                    }
                ],
                occurrences: vec![
                    ScipOccurrence {
                        range: vec![5, 0, 4],
                        symbol: "scip-python local . . user.py/User#".to_string(),
                        symbol_roles: Some(1),
                        enclosing_range: None,
                    }
                ],
            }
        ],
    };
    
    let (python_symbols, python_edges, python_occurrences) = scip_mapper.map_scip_to_ir(&python_scip, "py_test")?;
    
    // Should still parse correctly despite different SCIP format
    assert!(!python_symbols.is_empty(), "Should parse Python SCIP symbols");
    assert!(!python_occurrences.is_empty(), "Should parse Python SCIP occurrences");
    
    // Verify the symbols have reasonable defaults/mappings
    let symbol = &python_symbols[0];
    assert!(symbol.name.contains("User"));
    assert!(symbol.file_path.contains("user.py"));
    
    println!("✅ Multi-language SCIP processing successful");
    Ok(())
}

#[test]
fn test_scip_provenance_tracking() -> Result<()> {
    println!("📋 Testing SCIP provenance tracking...");
    
    let indexer_name = "custom-indexer";
    let indexer_version = "2.1.0";
    let scip_mapper = scip_mapper::ScipMapper::new(indexer_name, indexer_version);
    
    let scip_index = ScipIndex {
        metadata: ScipMetadata {
            tool_info: ScipToolInfo {
                name: indexer_name.to_string(),
                version: indexer_version.to_string(),
            },
            project_root: "file:///test".to_string(),
            text_document_encoding: Some(0),
        },
        documents: vec![
            ScipDocument {
                relative_path: "test.ts".to_string(),
                symbols: vec![
                    ScipSymbol {
                        symbol: "scip-typescript npm . . `test.ts`/TestClass#".to_string(),
                        documentation: None,
                        relationships: Some(vec![
                            ScipRelationship {
                                symbol: "scip-typescript npm . . `base.ts`/BaseClass#".to_string(),
                                is_implementation: Some(true),
                                is_reference: Some(false),
                            }
                        ]),
                    }
                ],
                occurrences: vec![],
            }
        ],
    };
    
    let (_symbols, edges, _occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "prov_test")?;
    
    // Verify provenance is tracked in edges
    assert!(!edges.is_empty());
    let edge = &edges[0];
    
    assert!(edge.provenance.contains_key("source"));
    let source_value = edge.provenance.get("source").unwrap();
    assert_eq!(source_value, &format!("{}@{}", indexer_name, indexer_version));
    assert_eq!(edge.resolution, Resolution::Semantic);
    
    println!("✅ SCIP provenance tracking successful");
    Ok(())
}