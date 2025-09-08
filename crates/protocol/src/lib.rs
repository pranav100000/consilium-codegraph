use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod version;
pub use version::{LanguageVersion, Version, VersionDetection};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Language {
    TypeScript,
    JavaScript,
    Python,
    Go,
    Rust,
    Java,
    C,
    Cpp,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    Variable,
    Type,
    Module,
    Package,
    Namespace,
    Enum,
    EnumMember,
    Struct,
    Trait,
    Constant,
    Field,
    Property,
    TypeAlias,
    Typedef,
    Union,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeType {
    Contains,
    Declares,
    Calls,
    Imports,
    Extends,
    Implements,
    Overrides,
    Returns,
    Reads,
    Writes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Resolution {
    Syntactic,
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OccurrenceRole {
    Reference,
    Read,
    Write,
    Call,
    Extend,
    Implement,
    Definition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolIR {
    pub id: String,
    pub lang: Language,
    pub lang_version: Option<Version>,  // Track the language version
    pub kind: SymbolKind,
    pub name: String,
    pub fqn: String,
    pub signature: Option<String>,
    pub file_path: String,
    pub span: Span,
    pub visibility: Option<String>,
    pub doc: Option<String>,
    pub sig_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeIR {
    pub edge_type: EdgeType,
    pub src: Option<String>,
    pub dst: Option<String>,
    pub file_src: Option<String>,
    pub file_dst: Option<String>,
    pub resolution: Resolution,
    pub meta: HashMap<String, serde_json::Value>,
    pub provenance: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OccurrenceIR {
    pub file_path: String,
    pub symbol_id: Option<String>,
    pub role: OccurrenceRole,
    pub span: Span,
    pub token: String,
}

impl SymbolIR {
    pub fn generate_id(commit_sha: &str, file_path: &str, lang: &Language, fqn: &str, sig_hash: &str) -> String {
        format!("repo://{}/{}/{}#sym({}:{}:{})", 
            commit_sha, 
            file_path.trim_start_matches('/'),
            "",
            format!("{:?}", lang).to_lowercase(),
            fqn,
            sig_hash
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_id_generation() {
        let id = SymbolIR::generate_id(
            "abc123",
            "src/main.rs",
            &Language::Rust,
            "mymod::MyStruct::new",
            "hash123"
        );
        assert!(id.starts_with("repo://abc123/src/main.rs"));
        assert!(id.contains("rust:mymod::MyStruct::new:hash123"));
    }

    #[test]
    fn test_serialize_deserialize() {
        let symbol = SymbolIR {
            id: "test_id".to_string(),
            lang: Language::TypeScript,
            lang_version: None,
            kind: SymbolKind::Function,
            name: "test".to_string(),
            fqn: "module.test".to_string(),
            signature: Some("(x: number) => number".to_string()),
            file_path: "test.ts".to_string(),
            span: Span {
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 10,
            },
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash: "abc".to_string(),
        };
        
        let json = serde_json::to_string(&symbol).unwrap();
        let deserialized: SymbolIR = serde_json::from_str(&json).unwrap();
        assert_eq!(symbol.id, deserialized.id);
    }
}