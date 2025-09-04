# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Consilium Codegraph is a Rust-based local CLI tool for building fast, multi-language code graphs with semantic enrichment. It uses Tree-sitter for syntactic analysis and SCIP indexers for semantic understanding.

## Core Architecture

The project consists of multiple crates in a Cargo workspace architecture:
- **core**: Main CLI application and orchestration
- **protocol**: IR types (SymbolIR, EdgeIR, OccurrenceIR) that serve as the common data model
- **store**: SQLite persistence layer for graph and semantic data
- **ts_harness**: Tree-sitter integration for TypeScript/JavaScript  
- **scip_mapper**: Maps SCIP indexer outputs to internal IR format

Data flows: Source files → Tree-sitter (syntactic) → SCIP indexers (semantic) → IR mapper → SQLite storage

## Key Commands

```bash
# Build the project
cargo build

# Run tests for all crates
cargo test

# Run tests for a specific crate
cargo test -p store
cargo test -p protocol

# Run the main binary
cargo run -- scan
cargo run -- show --symbol <FQN>
cargo run -- search "query"

# Check code formatting
cargo fmt -- --check

# Run linting
cargo clippy
```

## Database Schema

The system uses two SQLite databases:
- **graph.db**: Stores files, symbols, edges, occurrences, and commit snapshots
- **semantic.db**: Stores chunks, FTS5 indices, and embeddings (V2)

Database location: `.reviewbot/graph.db` and `.reviewbot/semantic.db`

## Implementation Status

Currently implementing V1 (Scan & Graph) with these milestones:
1. Bootstrap & Contracts - IR types and SQLite schemas
2. Gitignore-aware walker
3. Tree-sitter harness for TS/JS
4. SQLite writer with batching and idempotence
5. FQN rules and stable symbol IDs  
6. Basic CLI commands (scan, show, search)
7. Incremental indexing
8. Python & Go support
9. SCIP ingestion for semantic upgrades
10. Petgraph neighborhood queries

## Development Guidelines

When implementing new language support:
- Add Tree-sitter parser integration in a new crate
- Implement FQN normalization rules specific to the language
- Create golden test fixtures under `testdata/<lang>-basic/`
- Add SCIP indexer integration mapping to IR

When working with the IR layer:
- All symbols must have stable IDs using format: `repo://{sha}/{path}#sym({lang}:{fqn}:{sig_hash})`
- Edge resolution can be "syntactic" or "semantic"
- Always include provenance metadata for semantic edges

When modifying SQL schemas:
- Add migrations to maintain backwards compatibility
- Use WAL mode for better concurrent access
- Batch writes in transactions of 1-10k rows

## Testing Approach

- **Test Repository**: Use `./test_repo` for development testing - this should contain sample TypeScript, Python, and Go files
- **Golden tests**: Compare symbol/edge counts against expected values in `testdata/` repos
- **Idempotence tests**: Ensure re-running scan produces zero mutations
- **Integration tests**: Test full pipeline from source to query results
- Run specific golden tests: `make test-goldens` (once Makefile is created)

## Test Commands

When testing the CLI, use the test repository:
```bash
# Run scan on test repository
cargo run -p reviewbot -- --repo ./test_repo scan

# Show symbols from test repository
cargo run -p reviewbot -- --repo ./test_repo show --symbol <FQN>

# Search in test repository  
cargo run -p reviewbot -- --repo ./test_repo search "query"
```

## Performance Targets

- 100k LOC mixed repo: ≤60s cold index, ≤10s incremental
- Neighborhood expansion: ≤200ms after load
- Re-scans with no changes must perform zero DB mutations