# Bug Fix: Quoted Values in Database

## Problem

The SQLite database stores `kind` and `edge_type` values with literal quotes as part of the value:
- Database: `"Function"`, `"Class"`, `"Calls"`, `"Imports"`
- TypeScript expected: `Function`, `Class`, `Calls`, `Imports`

This caused:
1. **findSymbols()** with kind filter to return 0 results
2. **getStats()** to show quoted keys like `"Function": 291` instead of `Function: 291`
3. **Edge filtering** to miss call relationships
4. Only 2 call edges were found instead of 2548

## Root Cause

The database schema stores these values with quotes:
```sql
SELECT kind FROM symbol LIMIT 1;
-- Returns: "Function" (with literal quotes in the value)

SELECT edge_type FROM edge LIMIT 1;
-- Returns: "Calls" (with literal quotes in the value)
```

When TypeScript queries used:
```sql
WHERE kind = 'Function'  -- Looks for Function
```

It couldn't match `"Function"` (with quotes).

## Solution

Applied two-pronged fix:

### 1. Query Filtering - Handle Both Formats

Updated all WHERE clauses to match both quoted and unquoted values:

```typescript
// Before (broken):
WHERE s.kind = ?

// After (works):
WHERE (REPLACE(s.kind, '"', '') = ? OR s.kind = ?)
```

This ensures we can query with clean values like `"Function"` and match both:
- `"Function"` (with quotes)
- `Function` (without quotes)

### 2. Response Normalization - Return Clean Values

Strip quotes from all returned values:

```typescript
// In all symbol/edge mapping functions:
const normalizedKind = row.kind.replace(/^"|"$/g, '');
const normalizedType = row.edge_type.replace(/^"|"$/g, '');
```

## Files Updated

### code-graph-api.ts
- `getSymbol()` - Normalize kind in response
- `findSymbols()` - Handle quoted kinds in WHERE, normalize in response
- `getFileSymbols()` - Normalize kind in response
- `getCallers()` - Handle quoted edge types in WHERE
- `getCallees()` - Handle quoted edge types in WHERE
- `getEdges()` - Handle quoted edge types in WHERE, normalize in response
- `getStats()` - Normalize kind and edge_type in statistics

### code-graph.ts
- `rowToSymbol()` - Normalize kind in response
- `findSymbols()` - Handle quoted kinds in WHERE
- `findPath()` - Handle quoted edge types in WHERE
- `traverseCalls()` - Handle quoted edge types in WHERE (both callers and callees)
- `getStatistics()` - Normalize kind and edge_type in statistics

## Verification

Before fix:
```
Symbols by Kind:
  "Function": 291  ❌ Quoted keys
  "Class": 38

Edges by Type:
  "Calls": 2       ❌ Only 2 found

Finding Functions: 0  ❌ Filter didn't work
Finding Classes: 0    ❌ Filter didn't work
```

After fix:
```
Symbols by Kind:
  Function: 291    ✅ Clean keys
  Class: 38

Edges by Type:
  Calls: 2548      ✅ All 2548 found

Finding Functions: 291  ✅ Filter works
Finding Classes: 38     ✅ Filter works
```

## Testing

Run the test script to verify:
```bash
cd ts-client
npm run build
npx tsx test-fix.ts
```

Expected output:
- ✅ All symbols found with correct counts
- ✅ Clean kind and edge_type values (no quotes)
- ✅ Kind filtering works (Functions, Classes, etc.)
- ✅ Edge filtering works (2548 Calls edges)

## Future Considerations

This is a **workaround** for the database schema. Ideally:

1. **Fix at source**: Update the Rust indexer to not store quotes in kind/edge_type fields
2. **Database migration**: Strip quotes from existing values
3. **Schema validation**: Ensure enum values are stored without quotes

For now, the TypeScript client handles both formats gracefully.
