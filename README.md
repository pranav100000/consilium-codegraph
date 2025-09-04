# Consilium CodeGraph 🚀

A blazing-fast code graph builder with semantic enrichment, supporting multiple languages and providing powerful code intelligence features.

## Features

- **Multi-language support**: TypeScript/JavaScript, Python, Go
- **Semantic code graph**: Build relationships between functions, classes, and modules
- **Fast incremental indexing**: Only re-parse changed files
- **Full-text search**: Fuzzy search across all symbols with FTS5
- **Graph analysis**: Find callers, callees, cycles, and paths between symbols
- **SQLite storage**: Persistent, queryable code graph database
- **SCIP-compatible**: Export to Standard Code Intelligence Protocol format

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/consilium-codegraph.git
cd consilium-codegraph

# Build the project
cargo build --release

# Run tests
cargo test --workspace
```

## Usage

### Scanning a Repository

```bash
# Scan the current directory
reviewbot scan

# Scan a specific repository
reviewbot --repo /path/to/repo scan

# Scan at a specific commit
reviewbot --repo /path/to/repo scan --commit abc123
```

### Searching for Symbols

```bash
# Search for symbols by name
reviewbot search "getUserData"

# Limit results
reviewbot search "User" --limit 10
```

### Graph Analysis

```bash
# Find what calls a function
reviewbot graph callers "UserService.authenticate" --depth 2

# Find what a function calls
reviewbot graph callees "main" --depth 3

# Find cycles containing a symbol
reviewbot graph cycles "EventHandler.process"

# Find path between two symbols
reviewbot graph path --from "main" --to "DatabaseConnection.query"
```

### Show Repository Statistics

```bash
# Display repository statistics
reviewbot show stats

# Show all symbols in a file
reviewbot show symbols --file src/main.ts

# Search with full-text search
reviewbot show search --query "auth"
```

## Architecture

```
crates/
├── core/           # Main binary and CLI
├── store/          # SQLite storage and graph operations
├── protocol/       # SCIP protocol definitions
├── ts_harness/     # TypeScript/JavaScript parser
├── py_harness/     # Python parser
├── go_harness/     # Go parser
└── file_walker/    # File system traversal
```

## Database Schema

The code graph is stored in SQLite with the following main tables:

- `commit`: Git commits that have been indexed
- `file`: Files in each commit with content hashes
- `symbol`: All symbols (functions, classes, etc.)
- `edge`: Relationships between symbols (calls, imports, etc.)
- `occurrence`: Symbol occurrences in files
- `symbol_fts`: Full-text search index

## Performance

- **Incremental updates**: Only changed files are re-parsed
- **Indexed queries**: All common queries use database indexes
- **In-memory graph**: Graph operations use petgraph for speed
- **Parallel processing**: File parsing can be parallelized
- **Tested at scale**: Handles repositories with 1000+ files

## Testing

The project includes comprehensive test coverage:

- **Unit tests**: 100+ tests covering all modules
- **Integration tests**: End-to-end scanning and querying
- **Edge cases**: Unicode, malformed code, large files
- **Performance tests**: Benchmarks for large repositories

```bash
# Run all tests
cargo test --workspace

# Run specific test suite
cargo test -p store
cargo test -p ts_harness
cargo test -p py_harness
cargo test -p go_harness

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --workspace
```

## Language Support

### TypeScript/JavaScript
- Functions, classes, methods
- Imports/exports
- JSX/TSX components
- Async/await, generators
- Decorators

### Python
- Functions, classes, methods
- Import statements
- Decorators, properties
- Async/await, generators
- Type hints

### Go
- Functions, methods, interfaces
- Structs and fields
- Import statements
- Goroutines and channels
- Generic types (Go 1.18+)

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with Rust 🦀
- Uses tree-sitter for parsing
- Powered by SQLite with FTS5
- Graph operations via petgraph