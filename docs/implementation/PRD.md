# Product Requirements Document (PRD)

**Project:** Rust Code Graph + Semantic Enrichment (Greptile‑style)
**Mode:** Local CLI, single binary
**Backbone:** Rust core (Tree‑sitter for syntax) + **SCIP indexers** as semantic adapters via a thin IR mapper
**Phases:** V1 = Scan/Graph; V2 = Summaries & Embeddings

---

## 1) Objectives

* **V1: Scan & Graph.** Fast, correct code graph for multi‑language repos using Tree‑sitter (syntactic) and **SCIP** (semantic upgrades). Persist to SQLite with stable IDs. Provide basic CLI queries.
* **V2: Semantic Enrichment.** Generate concise summaries per symbol and embeddings for hybrid retrieval with FTS5.

**Non‑goals (for this PRD):** Hosting, queues, web APIs, PR comment generation, patch suggestions, IDE extensions.

---

## 2) Users & Use Cases

* **Developers/Reviewers:** Explore callers/callees/importers; search symbols and concepts (V2).
* **Tooling/Infra:** Dependable, local artifact for downstream analysis and experiments.

Representative tasks:

* V1: “List callers of `Auth.validateJWT` and impacted files.”
* V2: “Find where we sanitize user input before DB writes.”

---

## 3) Architecture (local‑only)

**Rust Core**

* CLI (clap), async orchestration (tokio), parallel CPU work (rayon)
* Syntax layer: Tree‑sitter parsers for breadth
* Semantic layer: **SCIP indexers** (external processes) → IR mapper → upgrade edges
* Graph ops: petgraph for neighborhood queries; rusqlite for persistence
* Search: SQLite FTS5 (lexical); vectors optional in V2

**Adapters (external, invoked by core)**

* Always **SCIP**: `scip‑typescript`, `scip‑python`, `scip‑go`, (others as added)
* Core spawns indexer per project/module, consumes emitted `.scip`, maps to IR

**On‑disk layout (per repo)**

```
.reviewbot/
  graph.db        # files, symbols, edges, occurrences, commits
  semantic.db     # chunks, FTS5, (optional) vectors
  cache/
    scip/<lang>/<project-hash>.scip   # raw index outputs (optional, for debug/cache)
    adapter/<lang>/<filehash>.json    # per-file parse cache (optional)
  logs/
```

---

## 4) Data Model (frozen IR)

**SymbolIR**

* `id`: `repo://{sha}/{path}#sym({lang}:{fqn}:{sig_hash})` (stable per commit)
* `lang`, `kind` (`function|method|class|interface|var|type|module|package|namespace`)
* `name`, `fqn`, `signature?`, `filePath`, `spanStart..spanEnd`, `visibility?`, `doc?`

**EdgeIR**

* `type`: `CONTAINS|DECLARES|CALLS|IMPORTS|EXTENDS|IMPLEMENTS|OVERRIDES|RETURNS|READS|WRITES`
* `src`, `dst` (Symbol IDs) or `fileSrc`, `fileDst` for file→file
* `resolution`: `syntactic|semantic`
* `meta`: receiver type, import flavor, generic arity, etc.
* `provenance`: `{source:"scip-typescript@x.y.z"}`

**OccurrenceIR**

* `filePath`, `symbolId?`, `role` (`ref|read|write|call|extend|implement`), `span`, `token`

**SQLite**

* `graph.db` tables: `commit_snapshot`, `file`, `symbol`, `edge`, `occurrence` (indexed by `commit_id`, `fqn`, `edge(src/dst/type)`)
* `semantic.db` tables: `chunk(id, commit_id, file_id, symbol_id, path, title, summary, text, identifiers, chunk_hash, summary_hash, embedding_hash)` + `chunk_fts` (FTS5)

---

## 5) SCIP → IR Mapper (thin but mandatory)

Purpose: normalize symbols/FQNs, standardize relationships, and insulate storage from SCIP changes.

* Parse SCIP symbols → `lang`, canonical `fqn`, pretty `signature`, `sig_hash`.
* Map SCIP occurrences + relationships to IR edges, set `resolution="semantic"`, attach `provenance`.
* Preserve raw `.scip` path for debugging; not required at query time.

---

## 6) CLI

```
# V1
reviewbot scan [--lang ts,py,go] [--commit <sha>] [--no-semantic] [--jobs N]
reviewbot show --symbol <FQN> [--callers|--callees|--importers] [--depth 1]
reviewbot search "query" [--k 20]    # FTS5 lexical (identifiers/docstrings)

# V2
reviewbot semantic build              # construct chunks (no LLM)
reviewbot semantic summarize --limit N --provider openai|anthropic|http
reviewbot semantic embed --batch N --provider openai|http
reviewbot search "concept" --hybrid   # BM25 + vectors + graph boost
```

---

# V1 — Scan & Graph

## Scope

* Tree‑sitter syntactic indexing for **TS/JS**, **Python**, **Go** (initial set)
* **SCIP indexers** for the same languages; ingest `.scip` files and upgrade edges
* Incremental indexing: dirty set from `git diff` + import graph for impacted files
* Persist graph to SQLite; expose neighborhood queries and basic search (FTS5)

## Functional Requirements

* Gitignore‑aware walker; per‑language project detection (tsconfig, go.mod, pyproject).
* Run Tree‑sitter pass → emit symbols/occurrences + *candidate* edges (CALL sites, IMPORTS) with `resolution="syntactic"`.
* If SCIP present, spawn indexer → parse `.scip` → map to IR → **replace/upgrade** to `resolution="semantic"` edges.
* Batch SQLite writes (WAL ON); single writer thread; concurrent readers.
* Deterministic symbol IDs; FQN rules per language (tested with goldens).

## Performance Targets

* 100k LOC mixed repo: **≤ 60s** cold index; **≤ 10s** incremental for small diffs.
* Neighborhood expansion (changed symbols + 1 hop): **≤ 200 ms** after load.

## Deliverables

* `reviewbot` binary with `scan`, `show`, `search` (FTS only)
* Ingestion of `scip‑typescript`, `scip‑python`, `scip‑go` (pinned versions)
* IR mapper crate + golden tests (tiny repos per language)

## Acceptance Criteria

* For language X: symbol/edge counts match expectations on goldens; semantic edges present when SCIP enabled.
* Re‑running `scan` with no changes performs **zero** DB mutations (idempotent).
* `search` returns identifiers and docstrings via FTS5.

## Risks & Mitigations

* Indexer runtime deps → `--no-semantic` flag; cache `.scip` by project hash; version pin in config.
* FQN mismatches → centralized normalization + tests.

## Milestones

* **W1:** Core CLI + SQLite + Tree‑sitter (TS/JS, Py, Go) syntactic scan
* **W2:** SCIP ingestion + IR upgrades; incremental indexing; FTS5; `show` subcommands

---

# V2 — Summaries & Embeddings

## Scope

* Build **chunks** per symbol: `title(FQN+sig) + identifiers + doc + excerpt + neighbor list`
* **Summaries:** 2–4 sentence LLM‑generated descriptions (cached by input hash)
* **Embeddings:** vectors over `title + summary + identifiers`; hybrid retrieval (BM25 + vector + graph boost)
* Staged passes default; `--eager` mode for small repos

## Functional Requirements

* Idempotence via hashes: `chunk_hash`, `summary_hash` (includes provider/model/version), `embedding_hash`.
* Provider adapters via traits:

  * `Summarizer::summarize(ChunkInput) -> String`
  * `Embedder::embed(Vec<String>) -> Vec<Vec<f32>>`
* Rate limits, retries with backoff, partial progress commit.
* Keep `chunk` and `chunk_fts` in sync within a transaction.
* Hybrid ranking: `α·cosine(vec) + β·bm25 + γ·graph_boost` with MMR de‑dupe by symbol/file.

## Performance & Cost Targets

* Summaries: **500–1,000 chunks/hour** per API key (tunable concurrency).
* Embeddings: batch **64–256** per request; 5k chunks in **< 5 min** typical.
* Cost guards: `--limit`, `--dry-run`, `--budget`.

## Deliverables

* CLI: `semantic build|summarize|embed`; provider configs; prompts
* Populated `semantic.db`; enhanced `search --hybrid`

## Acceptance Criteria

* Re‑runs with no edits do **zero** provider calls (hash cache works)
* Manual QA: top‑K results for concept queries look relevant on test repos
* Failures (429/5xx) recover on retry; partial results persist

## Risks & Mitigations

* API limits → global QPS; exponential backoff with jitter
* Token blowups → strict input truncation; windowed excerpts; identifier caps
* PII/secrets → scrubber before prompts; allowlist/denylist paths; local‑only mode honored

## Milestones

* **W1:** Chunk builder, summarizer (OpenAI) + embedder, hashing/caching
* **W2:** Hybrid retrieval, provider abstraction, docs/examples

---

## 7) Testing & Telemetry

* **Goldens:** per‑language repos (symbols, edges, FQNs); mapping tests for SCIP → IR
* **Metrics:** parse time/file, unresolved refs %, row counts, cache hit rate; V2 adds token/cost, QPS, error rates
* **Benchmarks:** cold vs incremental scan timings; search P95 latency with/without vectors

---

## 8) Future (out of scope)

* PR review engine (detectors + evidence‑anchored comments)
* Policies/config enforcement
* External graph/vector DB backends (Neo4j/pgvector/Qdrant) via pluggable interfaces
* IDE/CI integrations and SARIF export
