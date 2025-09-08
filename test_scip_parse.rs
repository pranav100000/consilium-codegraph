use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TestScipOccurrence {
    pub range: Vec<i32>,
    pub symbol: String,
    pub symbol_roles: i32,
    pub enclosing_range: Option<Vec<i32>>,
}

fn main() {
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

    match serde_json::from_str::<Vec<TestScipOccurrence>>(json_data) {
        Ok(occurrences) => {
            println!("✅ Successfully parsed {} occurrences", occurrences.len());
            for (i, occ) in occurrences.iter().enumerate() {
                println!("  {}: {} (roles: {})", i, occ.symbol, occ.symbol_roles);
            }
        },
        Err(e) => {
            println!("❌ Failed to parse: {}", e);
        }
    }
}