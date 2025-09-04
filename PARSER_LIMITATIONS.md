# Parser Limitations Identified by Strict Tests

This document lists all parser limitations discovered through strict testing. These represent areas where the current implementation does not fully extract all symbols and relationships from the code.

## Recent Improvements (Fixed)

The following limitations have been addressed:

### C++ Parser
- ✅ Namespace parsing now implemented
- ✅ Constructor/destructor detection working
- ✅ Virtual/override/final specifiers tracked
- ✅ Typedef and using alias support added
- ✅ Union types now parsed
- ✅ Operator overloading names normalized
- ✅ Template parameters captured for functions and classes
- ✅ Nested classes fully supported
- ✅ Lambda expressions tracked

### Java Parser  
- ✅ Field visibility properly captured (public/private/protected/package)
- ✅ Method visibility tracked
- ✅ Multiple interface extension fixed
- ✅ Constructor signatures include parameter types
- ✅ Generic type parameters captured for classes/interfaces/methods
- ✅ Annotation usage tracked
- ✅ Exception specifications captured in method signatures
- ✅ Inner/nested classes fully supported
- ✅ Lambda expressions and method references handled

## C++ Parser Limitations

### 1. Namespace Parsing Not Implemented
**Test**: `test_exact_namespace_parsing`
**Issue**: Parser finds 0 namespaces when expecting 3
**Details**: 
- Nested namespaces (`namespace outer { namespace inner { } }`) are not parsed
- C++17 concatenated namespaces (`namespace a::b::c { }`) are not parsed
- No `SymbolKind::Namespace` symbols are created

### 2. Method Visibility Not Tracked
**Test**: `test_exact_access_modifiers`
**Issue**: Access modifiers (public, private, protected) are not properly tracked
**Details**:
- Fields have visibility tracking but it's not working correctly
- Methods don't track visibility at all
- Access specifier sections in classes are not parsed

### 3. Constructor Detection Missing
**Test**: `test_exact_template_parsing`
**Issue**: Constructors are not identified as methods
**Details**:
- Constructors (methods with same name as class) are not parsed
- This affects both regular and template class constructors
- Destructor parsing is likely also missing

### 4. Template Declarations Not Fully Parsed
**Test**: `test_exact_template_parsing`
**Issue**: Template parameters and specializations are ignored
**Details**:
- Template parameter lists are skipped
- Template specializations are not distinguished
- Variadic templates are not handled

### 5. Virtual/Override Methods Not Distinguished
**Test**: `test_exact_inheritance_edges`
**Issue**: Virtual methods and override specifiers are not tracked
**Details**:
- `virtual` keyword is ignored
- `override` and `final` specifiers are not parsed
- Pure virtual methods (`= 0`) are not distinguished

### 6. Typedef/Using Aliases Not Parsed
**Test**: `test_exact_typedef_and_using`
**Issue**: Type aliases are completely skipped
**Details**:
- `typedef` statements are ignored
- `using` type aliases are ignored
- Template aliases are not supported

### 7. Union Types Not Parsed
**Test**: `test_exact_struct_parsing`
**Issue**: Union declarations are skipped
**Details**:
- `union` keyword is not recognized
- Union members are not extracted

### 8. Operator Overloading Not Fully Supported
**Test**: `test_exact_operator_overloading`
**Issue**: Overloaded operators are not properly identified
**Details**:
- Operator names like `operator+` are not parsed correctly
- Friend operators are missed
- Conversion operators are not handled

### 9. Multiple Inheritance Not Fully Tracked
**Test**: `test_exact_inheritance_edges`
**Issue**: Only first base class is tracked in multiple inheritance
**Details**:
- Parser only captures first base class in inheritance list
- Private/protected inheritance is not distinguished
- Virtual inheritance is not handled

### 10. Include Path Resolution Incomplete
**Test**: `test_exact_include_edges`
**Issue**: Include paths are captured but not normalized
**Details**:
- Angle bracket vs quote includes not distinguished
- Relative paths not resolved
- System headers not differentiated

## Java Parser Limitations

### 1. Field Visibility Not Captured
**Test**: `test_exact_class_parsing`
**Issue**: Field visibility modifiers are not extracted
**Details**:
- Fields are found but their visibility (private/protected/public) is None
- Static and final modifiers are not tracked
- Volatile and transient modifiers are ignored

### 2. Multiple Interface Extension Not Tracked
**Test**: `test_exact_interface_implementation`
**Issue**: Only first extended interface is captured
**Details**:
- When interface extends multiple interfaces, only first is tracked
- `extends A, B, C` only captures edge to A
- Same issue likely affects `implements` with multiple interfaces

### 3. Constructor Overloading Not Distinguished
**Test**: `test_exact_class_parsing`
**Issue**: Multiple constructors with different signatures not properly tracked
**Details**:
- Constructor signatures are not captured
- Overloaded constructors may be deduplicated incorrectly
- Parameter types are not included in method signatures

### 4. Generic Type Parameters Ignored
**Test**: `test_exact_generic_parsing`
**Issue**: Generic type information is lost
**Details**:
- Type parameters on classes/methods are ignored
- Bounded types (`T extends Comparable`) not captured
- Wildcards (`? extends`, `? super`) not tracked

### 5. Annotation Metadata Not Captured
**Test**: `test_exact_annotation_parsing`
**Issue**: Annotations are skipped entirely
**Details**:
- `@Override`, `@Deprecated` etc. are not tracked
- Custom annotations are not parsed as interfaces
- Annotation parameters are ignored

### 6. Inner/Nested Classes Partially Supported
**Test**: `test_exact_inner_class_parsing`
**Issue**: Inner class hierarchy is incomplete
**Details**:
- Static nested classes may not be distinguished
- Local classes inside methods are missed
- Anonymous classes are not tracked

### 7. Record Components Not Extracted
**Test**: `test_exact_record_parsing`
**Issue**: Record structure is not fully parsed
**Details**:
- Record components are not tracked as fields
- Compact constructors are missed
- Record implements clauses may not work

### 8. Exception Specifications Ignored
**Test**: `test_exact_exception_handling`
**Issue**: Throws clauses are not tracked
**Details**:
- `throws IOException, SQLException` is ignored
- Exception hierarchy relationships not captured
- Try-with-resources variables not tracked

### 9. Static Initializer Blocks Missed
**Test**: `test_exact_static_members`
**Issue**: Static and instance initializer blocks are not parsed
**Details**:
- `static { }` blocks are ignored
- Instance initializer `{ }` blocks are ignored
- Order of initialization is not tracked

### 10. Enum Methods and Fields Not Fully Captured
**Test**: `test_exact_enum_parsing`
**Issue**: Enum structure is simplified
**Details**:
- Enum constant parameters are ignored
- Methods in enum constants are missed
- Enum constructors not properly identified

## Common Limitations Across Languages

### 1. Method Parameter Details Lost
- Parameter names often missing
- Parameter types not fully captured
- Default parameter values ignored
- Varargs not distinguished

### 2. Import/Include Resolution
- Imported symbols not linked to their definitions
- Import aliases not tracked
- Wildcard imports not expanded

### 3. Documentation Comments
- Javadoc/Doxygen comments not extracted
- Documentation tags not parsed
- Example code in comments ignored

### 4. Macro/Preprocessor Handling
- C++ macros completely ignored
- Conditional compilation not handled
- Macro expansions not tracked

### 5. Lambda/Closure Support
- Lambda expressions not fully parsed
- Captured variables not tracked
- Lambda types not inferred

## Priority Improvements

Based on impact and frequency of use, the top priorities for improvement are:

1. **Fix visibility modifiers** - Critical for understanding encapsulation
2. **Complete inheritance tracking** - Essential for OOP analysis  
3. **Parse constructors/destructors** - Fundamental methods
4. **Handle namespaces/packages fully** - Required for proper FQNs
5. **Track method signatures completely** - Needed for overload resolution
6. **Support type aliases** - Common in modern C++
7. **Parse annotations/attributes** - Important metadata
8. **Handle templates/generics properly** - Core language features
9. **Track exception specifications** - Important for API contracts
10. **Support inner/nested classes** - Common pattern

## Testing Recommendations

1. Keep the strict tests even though they fail - they document expected behavior
2. Add "lenient" test variants that pass with current limitations
3. Create parser capability matrices showing feature support by language
4. Add test fixtures from real codebases to find more edge cases
5. Benchmark parser performance on large files
6. Test incremental parsing with specific change patterns

## Implementation Notes

Most limitations stem from:
1. Incomplete tree-sitter query patterns
2. Missing node type handlers in the visitor
3. Oversimplified symbol extraction logic
4. Lack of semantic analysis phase
5. No symbol resolution pass

Fixing these will require:
1. Studying tree-sitter grammar files more deeply
2. Adding more node type cases to the parse tree visitor
3. Implementing multi-pass analysis
4. Building symbol tables for resolution
5. Adding semantic analysis for type information