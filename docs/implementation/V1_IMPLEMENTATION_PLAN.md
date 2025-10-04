awesome—here’s a crisp, bite-sized plan to deliver **V1 (Scan & Graph)** in small, verifiable steps. Each step has: goal → what to build → how to test (automated + quick manual checks) → definition of done.

---

# 0) Bootstrap & Contracts (day-zero sanity)

**Goal:** lock interfaces so work can parallelize.

* **Build:**

  * Cargo workspace (`crates/core`, `crates/protocol`, `crates/store`, `crates/ts_harness` (Tree-sitter TS), `crates/scip_mapper`).
  * IR types in `protocol` (`SymbolIR`, `EdgeIR`, `OccurrenceIR` + enums, serde).
  * SQLite schemas & migrations in `store` (`graph.db`: commit/file/symbol/edge/occurrence).
* **Test:** unit tests compile IR; migration runs and tables exist.

  * `cargo test -p store`
  * `sqlite3 .reviewbot/graph.db '.schema symbol'` shows expected columns.
* **Done when:** schema + IR compile; `cargo build` succeeds; a no-op `reviewbot scan` prints “nothing indexed”.

---

# 1) Gitignore-aware walker + commit snapshot

**Goal:** deterministic file list & a commit id to hang rows off.

* **Build:** repo root detector; `.gitignore` + common vendor filters via `ignore` crate; `git rev-parse HEAD` wrapper; content hash per file.
* **Test:**

  * Unit: walker respects `.gitignore` (fixture repo).
  * Manual: `reviewbot scan --no-write` prints N files; rerun is stable.
* **Done when:** file count equals `git ls-files | wc -l` (within filters); commit snapshot inserted.

---

# 2) Tree-sitter harness (TS/JS first) → syntactic symbols & occurrences

**Goal:** get fast, language-agnostic structure.

* **Build:** integrate `tree-sitter-javascript` (covers TS/JS); queries for: functions, classes, methods, imports, call expressions, identifiers + spans. Emit **syntactic** `SymbolIR` + `OccurrenceIR` and **candidate** edges: `CONTAINS`, `DECLARES`, `IMPORTS(file→file)`, `CALLS` (callee by textual name only for now), all with `resolution="syntactic"`.
* **Test:** tiny fixture:

  ```ts
  // src/a.ts
  export function foo(x: number) { return x+1 }
  // src/b.ts
  import { foo } from "./a"; export const z = foo(41);
  ```

  * Golden test: expect 2 function/var symbols, 1 IMPORTS(file→file), 1 CALLS (syntactic).
* **Done when:** `reviewbot scan` writes rows; `SELECT COUNT(*) FROM symbol` and `FROM edge WHERE resolution='syntactic'` match goldens.

---

# 3) SQLite writer (batched, WAL) + idempotence

**Goal:** durable, fast writes; reruns do nothing when nothing changed.

* **Build:** single writer thread; batched inserts (1–10k rows/txn); `INSERT OR REPLACE` policies tied to `(commit_id, fqn)` and `(src,dst,type,commit_id)`; WAL ON.
* **Test:** run `scan` twice on the same commit—second run does 0 mutations.

  * Assert in tests: row counts unchanged; track a “mutations” counter.
* **Done when:** re-scan is idempotent; basic timings logged.

---

# 4) FQN rules & stable symbol IDs

**Goal:** lock naming so cross-commit joins work.

* **Build:** canonical FQN builders for TS/JS:
  `pkg.module.Class#method(argTypes)->ret` | `pkg.module.func(argTypes)->ret` (for now, types omitted—will come from SCIP later); `sig_hash` placeholder from param count/name.
* **Test:** goldens for dotted vs relative imports, default exports, index files, re-exports.
* **Done when:** FQNs are deterministic; IDs are stable across re-scans.

---

# 5) Basic CLI: `scan`, `show`, `search(FTS)`

**Goal:** observable value early.

* **Build:**

  * `reviewbot scan` (current HEAD).
  * `reviewbot show --symbol <FQN> --callers|--callees|--importers --depth 1`.
  * Add `semantic.db` with `chunk_fts` and, for V1, index **identifiers + doc comments** scraped from syntax (no LLM).
  * `reviewbot search "identifier or phrase"` → FTS over identifiers/comments.
* **Test:**

  * Unit: SQL for callers/callees recursive CTE (depth 1).
  * Manual: `reviewbot search foo` returns `src/a.ts`.
* **Done when:** commands work on fixture repo; outputs match expected files/symbols.

---

# 6) Incremental indexing (dirty/impacted set)

**Goal:** fast re-scans on small diffs.

* **Build:**

  * Dirty set = changed files from `git diff --name-only <last_scanned_sha>..HEAD`.
  * Impacted = + dependents via `IMPORTS(file→file)` (syntactic) one hop.
  * Re-parse only dirty + impacted; delete/replace prior rows for those files.
* **Test:** modify `a.ts`; expect `a.ts` + `b.ts` reprocessed; total time drops.

  * Assert: only edges/files for those paths changed.
* **Done when:** re-scan after 1-file change updates only necessary rows.

---

# 7) Add Python & Go Tree-sitter syntactic coverage

**Goal:** prove multi-language breadth.

* **Build:** `tree-sitter-python`, `tree-sitter-go` queries for defs/imports/calls; language detection by extension; shared IR emission.
* **Test:** tiny fixtures for each (func defines, imports, calls) with goldens on counts.
* **Done when:** `reviewbot scan` on mixed repo stores symbols/edges for all 3 langs.

---

# 8) SCIP ingestion (TypeScript first) + thin IR mapper

**Goal:** upgrade edges to **semantic** truth for TS/JS.

* **Build:**

  * Runner for `scip-typescript` per tsconfig; cache `.scip` under `.reviewbot/cache/scip/...`.
  * `scip_mapper`: parse `.scip` (Rust crate) → map **symbols, occurrences, relationships** to IR, set `resolution="semantic"`, attach `provenance`.
  * Replace syntactic edges when semantic ones exist (or keep both with precedence).
* **Test:** same TS fixture; now expect `CALLS` with resolved callee symbol ID; `IMPORTS` upgraded; add an `OVERRIDES` case in a tiny OOP fixture.

  * Unit: mapper translates SCIP relationships to our edge types.
  * Manual: `SELECT type,resolution,COUNT(*) FROM edge GROUP BY 1,2;` shows both buckets; semantic present.
* **Done when:** semantic edges appear for TS; syntactic remains for Py/Go.

---

# 9) Incremental + SCIP (dirty project re-index)

**Goal:** semantic upgrades don’t blow up runtime.

* **Build:** detect which tsconfig/project contains dirty files; re-run `scip-typescript` only for those; merge/upgrade only affected symbols/edges.
* **Test:** change a single TS file; verify only that project’s `.scip` is refreshed and DB deltas are scoped.
* **Done when:** incremental scan with SCIP is fast and scoped; caches hit on no-op.

---

# 10) Petgraph neighborhood queries (in-memory subgraph)

**Goal:** fast multi-hop queries for the changed set.

* **Build:** loader that pulls just the PR-scope subgraph (changed symbols + 1 hop) into `petgraph`; functions for callers/callees/importers (depth 1–2).
* **Test:** goldens for expected neighborhoods in fixtures; check latency logs (sub-200ms).
* **Done when:** `reviewbot show --callers --depth 2` returns expected set quickly.

---

# 11) Diagnostics, logging, and safety rails

**Goal:** debuggable & safe by default.

* **Build:** run logs per phase (walk, parse, write, scip map); mutation counts; timings. Flags: `--no-semantic`, `--jobs N`, `--lang ts,py,go`, `--commit <sha>`.
* **Test:** unit for logger guards; manual toggles work.
* **Done when:** logs make regressions obvious; flags control work as advertised.

---

# 12) Golden repos & CI matrix

**Goal:** repeatable verification; catch regressions early.

* **Build:**

  * `testdata/ts-basic`, `ts-oop`, `py-basic`, `go-basic` small repos with expected JSON summaries for symbol/edge counts and specific FQNs.
  * CI jobs:

    1. **Syntax-only**: run `reviewbot scan --no-semantic` → assert goldens.
    2. **With SCIP TS**: install scip-typescript, run scan → assert upgraded edges appear; counts match.
* **Test:** done by CI; local `make test-goldens`.
* **Done when:** full matrix is green; adding a new language/feature requires updating goldens.

---

## Quick “definition of done” checklist for V1

* `scan`, `show`, `search` commands implemented.
* Tree-sitter syntactic coverage for TS/JS, Python, Go.
* **TS semantic upgrades via SCIP** present (edges marked `resolution="semantic"` with provenance).
* Incremental indexing works (dirty + impacted).
* SQLite idempotence on no-op scans.
* Golden tests cover symbol/edge counts, FQNs, and basic neighborhoods.
* Logs and flags make behavior observable and controllable.

---

## Handy verification snippets

* **Counts by edge type/resolution**

  ```
  sqlite3 .reviewbot/graph.db "
    SELECT type,resolution,COUNT(*) FROM edge GROUP BY 1,2 ORDER BY 1,2;
  "
  ```

* **Callers of a symbol (one hop)**

  ```
  sqlite3 .reviewbot/graph.db "
    SELECT s2.fqn
    FROM edge e
    JOIN symbol s1 ON s1.id = e.dst_symbol
    JOIN symbol s2 ON s2.id = e.src_symbol
    WHERE e.type='CALLS' AND s1.fqn='pkg.module.foo';
  "
  ```

* **Idempotence check**

  ```
  reviewbot scan && reviewbot scan
  # Expect log: mutations=0 on second run
  ```

* **Search sanity**

  ```
  reviewbot search "validate JWT"   # should surface auth-related identifiers/docs
  ```

---

If you want, I can drop a tiny **task board** (issue list) mapped to these steps, plus the **fixture repos** and initial goldens so you can start checking boxes immediately.
