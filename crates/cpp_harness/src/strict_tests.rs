#[cfg(test)]
mod strict_tests {
    use crate::*;
    use anyhow::Result;
    use protocol::{EdgeType, SymbolKind, OccurrenceRole};
    
    #[test]
    fn test_exact_namespace_parsing() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
namespace outer {
    namespace middle {
        namespace inner {
            class TestClass {
                int field1;
                double field2;
            public:
                void method1();
                int method2(int x);
            };
        }
    }
}
"#;
        
        let (symbols, _, occurrences) = harness.parse("test.cpp", source)?;
        
        // Exact symbol counts
        let namespaces = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Namespace)
            .collect::<Vec<_>>();
        assert_eq!(namespaces.len(), 3, "Should find exactly 3 namespaces");
        
        // Check exact namespace names and FQNs
        assert!(namespaces.iter().any(|s| s.name == "outer" && s.fqn == "outer"));
        assert!(namespaces.iter().any(|s| s.name == "middle" && s.fqn == "outer::middle"));
        assert!(namespaces.iter().any(|s| s.name == "inner" && s.fqn == "outer::middle::inner"));
        
        // Exact class parsing
        let classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .collect::<Vec<_>>();
        assert_eq!(classes.len(), 1, "Should find exactly 1 class");
        
        let test_class = &classes[0];
        assert_eq!(test_class.name, "TestClass");
        assert_eq!(test_class.fqn, "outer::middle::inner::TestClass");
        assert_eq!(test_class.file_path, "test.cpp");
        
        // Exact field parsing
        let fields = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Field)
            .collect::<Vec<_>>();
        assert_eq!(fields.len(), 2, "Should find exactly 2 fields");
        
        assert!(fields.iter().any(|f| f.name == "field1" && 
            f.fqn == "outer::middle::inner::TestClass::field1"));
        assert!(fields.iter().any(|f| f.name == "field2" && 
            f.fqn == "outer::middle::inner::TestClass::field2"));
        
        // Exact method parsing
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect::<Vec<_>>();
        assert_eq!(methods.len(), 2, "Should find exactly 2 methods");
        
        assert!(methods.iter().any(|m| m.name == "method1"));
        assert!(methods.iter().any(|m| m.name == "method2"));
        
        // Verify occurrences
        assert_eq!(occurrences.len(), symbols.len(), 
            "Each symbol should have exactly one occurrence");
        
        for occ in &occurrences {
            assert_eq!(occ.role, OccurrenceRole::Definition,
                "All occurrences should be definitions");
            assert!(occ.symbol_id.is_some(), "Occurrence should have symbol_id");
        }
        
        Ok(())
    }
    
    #[test]
    fn test_exact_inheritance_edges() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
class Base {
public:
    virtual void baseMethod() = 0;
};

class Derived : public Base {
public:
    void baseMethod() override;
    void derivedMethod();
};

class MultipleDerived : public Base, private std::enable_shared_from_this<MultipleDerived> {
public:
    void baseMethod() override;
};
"#;
        
        let (symbols, edges, _) = harness.parse("test.cpp", source)?;
        
        // Exact class count
        let classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .collect::<Vec<_>>();
        assert_eq!(classes.len(), 3, "Should find exactly 3 classes");
        
        // Verify class names
        let class_names: Vec<&str> = classes.iter().map(|c| c.name.as_str()).collect();
        assert!(class_names.contains(&"Base"));
        assert!(class_names.contains(&"Derived"));
        assert!(class_names.contains(&"MultipleDerived"));
        
        // Exact edge verification
        let extends_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Extends)
            .collect();
        
        assert!(extends_edges.len() >= 2, "Should have at least 2 extends edges");
        
        // Check specific inheritance relationships
        assert!(extends_edges.iter().any(|e| 
            e.src.as_ref().map(|s| s.contains("Derived")).unwrap_or(false) &&
            e.dst.as_ref().map(|d| d.contains("Base")).unwrap_or(false)
        ), "Derived should extend Base");
        
        assert!(extends_edges.iter().any(|e|
            e.src.as_ref().map(|s| s.contains("MultipleDerived")).unwrap_or(false) &&
            e.dst.as_ref().map(|d| d.contains("Base")).unwrap_or(false)
        ), "MultipleDerived should extend Base");
        
        // Verify methods
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect::<Vec<_>>();
        
        assert!(methods.len() >= 4, "Should find at least 4 methods");
        
        // Check virtual method presence
        assert!(methods.iter().any(|m| m.name == "baseMethod"));
        assert!(methods.iter().any(|m| m.name == "derivedMethod"));
        
        Ok(())
    }
    
    #[test]
    fn test_exact_function_calls() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
void helper(int x) {
    printf("Value: %d\n", x);
}

void caller() {
    helper(42);
    helper(100);
    printf("Done\n");
}

int main() {
    caller();
    helper(5);
    return 0;
}
"#;
        
        let (symbols, edges, occurrences) = harness.parse("test.cpp", source)?;
        
        // Exact function count
        let functions = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .collect::<Vec<_>>();
        assert_eq!(functions.len(), 3, "Should find exactly 3 functions");
        
        // Verify function names
        assert!(functions.iter().any(|f| f.name == "helper"));
        assert!(functions.iter().any(|f| f.name == "caller"));
        assert!(functions.iter().any(|f| f.name == "main"));
        
        // Exact call edges
        let call_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Calls)
            .collect();
        
        assert!(call_edges.len() >= 5, "Should have at least 5 call edges");
        
        // Verify specific calls
        assert!(call_edges.iter().any(|e|
            e.src.as_ref().map(|s| s.contains("caller")).unwrap_or(false) &&
            e.dst.as_ref().map(|d| d == "helper").unwrap_or(false)
        ), "caller should call helper");
        
        assert!(call_edges.iter().any(|e|
            e.src.as_ref().map(|s| s.contains("main")).unwrap_or(false) &&
            e.dst.as_ref().map(|d| d == "caller").unwrap_or(false)
        ), "main should call caller");
        
        // Check occurrences
        let references = occurrences.iter()
            .filter(|o| o.role == OccurrenceRole::Reference)
            .count();
        assert!(references >= 5, "Should have at least 5 references for function calls");
        
        Ok(())
    }
    
    #[test]
    fn test_exact_template_parsing() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
template<typename T>
class Container {
    T value;
public:
    Container(T v) : value(v) {}
    T get() const { return value; }
    void set(T v) { value = v; }
};

template<typename T, typename U>
struct Pair {
    T first;
    U second;
};

template<>
class Container<int> {
    int special_value;
public:
    int get() const { return special_value * 2; }
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Find all classes and structs
        let classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class || s.kind == SymbolKind::Struct)
            .collect::<Vec<_>>();
        
        assert!(classes.len() >= 2, "Should find at least 2 class/struct definitions");
        
        // Check Container class
        let container = symbols.iter()
            .find(|s| s.name == "Container" && s.kind == SymbolKind::Class);
        assert!(container.is_some(), "Should find Container class");
        
        // Check Pair struct
        let pair = symbols.iter()
            .find(|s| s.name == "Pair" && s.kind == SymbolKind::Struct);
        assert!(pair.is_some(), "Should find Pair struct");
        
        // Check methods in Container
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method && 
                       s.fqn.contains("Container"))
            .collect::<Vec<_>>();
        
        assert!(methods.iter().any(|m| m.name == "Container"), "Should find constructor");
        assert!(methods.iter().any(|m| m.name == "get"), "Should find get method");
        assert!(methods.iter().any(|m| m.name == "set"), "Should find set method");
        
        // Check fields
        let fields = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Field)
            .collect::<Vec<_>>();
        
        assert!(fields.iter().any(|f| f.name == "value"), "Should find value field");
        assert!(fields.iter().any(|f| f.name == "first"), "Should find first field");
        assert!(fields.iter().any(|f| f.name == "second"), "Should find second field");
        
        Ok(())
    }
    
    #[test]
    fn test_exact_enum_parsing() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
enum Color {
    RED,
    GREEN,
    BLUE
};

enum class Status : int {
    OK = 0,
    ERROR = 1,
    WARNING = 2,
    CRITICAL = 3
};

enum Flags {
    FLAG_A = 1 << 0,
    FLAG_B = 1 << 1,
    FLAG_C = 1 << 2,
    FLAG_ALL = FLAG_A | FLAG_B | FLAG_C
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Exact enum count
        let enums = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Enum)
            .collect::<Vec<_>>();
        assert_eq!(enums.len(), 3, "Should find exactly 3 enums");
        
        // Check enum names
        assert!(enums.iter().any(|e| e.name == "Color"));
        assert!(enums.iter().any(|e| e.name == "Status"));
        assert!(enums.iter().any(|e| e.name == "Flags"));
        
        // Check enum members
        let members = symbols.iter()
            .filter(|s| s.kind == SymbolKind::EnumMember)
            .collect::<Vec<_>>();
        
        assert!(members.len() >= 10, "Should find at least 10 enum members");
        
        // Check specific members
        let color_members = ["RED", "GREEN", "BLUE"];
        for member in color_members {
            assert!(members.iter().any(|m| m.name == member),
                "Should find {} member", member);
        }
        
        let status_members = ["OK", "ERROR", "WARNING", "CRITICAL"];
        for member in status_members {
            assert!(members.iter().any(|m| m.name == member),
                "Should find {} member", member);
        }
        
        Ok(())
    }
    
    #[test]
    fn test_exact_include_edges() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
#include <iostream>
#include <vector>
#include <string>
#include "local_header.h"
#include "../relative/path.h"

using namespace std;

class Test {
    vector<string> items;
};
"#;
        
        let (_, edges, _) = harness.parse("test.cpp", source)?;
        
        // Check import edges
        let import_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .collect();
        
        assert_eq!(import_edges.len(), 5, "Should find exactly 5 include directives");
        
        // Check specific includes
        let includes = import_edges.iter()
            .filter_map(|e| e.dst.as_ref())
            .collect::<Vec<_>>();
        
        assert!(includes.iter().any(|i| i.contains("iostream")));
        assert!(includes.iter().any(|i| i.contains("vector")));
        assert!(includes.iter().any(|i| i.contains("string")));
        assert!(includes.iter().any(|i| i.contains("local_header.h")));
        assert!(includes.iter().any(|i| i.contains("relative/path.h")));
        
        Ok(())
    }
    
    #[test]
    fn test_exact_access_modifiers() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
class AccessTest {
private:
    int private_field;
    void private_method();
    
protected:
    int protected_field;
    void protected_method();
    
public:
    int public_field;
    void public_method();
    
private:
    int another_private;
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Check class
        let class = symbols.iter()
            .find(|s| s.name == "AccessTest" && s.kind == SymbolKind::Class);
        assert!(class.is_some(), "Should find AccessTest class");
        
        // Check fields with visibility
        let fields = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Field)
            .collect::<Vec<_>>();
        
        assert_eq!(fields.len(), 4, "Should find exactly 4 fields");
        
        // Check specific visibility
        let private_field = fields.iter()
            .find(|f| f.name == "private_field");
        assert!(private_field.is_some());
        assert_eq!(private_field.unwrap().visibility.as_deref(), Some("private"));
        
        let protected_field = fields.iter()
            .find(|f| f.name == "protected_field");
        assert!(protected_field.is_some());
        assert_eq!(protected_field.unwrap().visibility.as_deref(), Some("protected"));
        
        let public_field = fields.iter()
            .find(|f| f.name == "public_field");
        assert!(public_field.is_some());
        assert_eq!(public_field.unwrap().visibility.as_deref(), Some("public"));
        
        // Check methods with visibility
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect::<Vec<_>>();
        
        assert_eq!(methods.len(), 3, "Should find exactly 3 methods");
        
        Ok(())
    }
    
    #[test]
    fn test_exact_operator_overloading() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
class Vector {
    double x, y;
public:
    Vector(double x, double y) : x(x), y(y) {}
    
    Vector operator+(const Vector& other) const {
        return Vector(x + other.x, y + other.y);
    }
    
    Vector& operator+=(const Vector& other) {
        x += other.x;
        y += other.y;
        return *this;
    }
    
    bool operator==(const Vector& other) const {
        return x == other.x && y == other.y;
    }
    
    friend std::ostream& operator<<(std::ostream& os, const Vector& v);
};

std::ostream& operator<<(std::ostream& os, const Vector& v) {
    return os << "(" << v.x << ", " << v.y << ")";
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Find class
        let vector_class = symbols.iter()
            .find(|s| s.name == "Vector" && s.kind == SymbolKind::Class);
        assert!(vector_class.is_some(), "Should find Vector class");
        
        // Check operators
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method || s.kind == SymbolKind::Function)
            .collect::<Vec<_>>();
        
        // Should find constructor and operator overloads
        assert!(methods.iter().any(|m| m.name == "Vector"), "Should find constructor");
        assert!(methods.iter().any(|m| m.name.contains("operator+")), "Should find operator+");
        assert!(methods.iter().any(|m| m.name.contains("operator+=")), "Should find operator+=");
        assert!(methods.iter().any(|m| m.name.contains("operator==")), "Should find operator==");
        assert!(methods.iter().any(|m| m.name.contains("operator<<")), "Should find operator<<");
        
        // Check fields
        let fields = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Field)
            .collect::<Vec<_>>();
        
        assert_eq!(fields.len(), 2, "Should find exactly 2 fields (x and y)");
        assert!(fields.iter().any(|f| f.name == "x"));
        assert!(fields.iter().any(|f| f.name == "y"));
        
        Ok(())
    }
    
    #[test]
    fn test_exact_typedef_and_using() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
typedef int Integer;
typedef void (*FunctionPointer)(int);
typedef struct {
    int x;
    int y;
} Point;

using String = std::string;
using IntVector = std::vector<int>;
using Callback = std::function<void(int)>;

template<typename T>
using SharedPtr = std::shared_ptr<T>;
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Check for type aliases
        let type_aliases = symbols.iter()
            .filter(|s| s.kind == SymbolKind::TypeAlias || 
                       s.kind == SymbolKind::Typedef)
            .collect::<Vec<_>>();
        
        // Should find at least some type definitions
        assert!(!type_aliases.is_empty(), "Should find type aliases/typedefs");
        
        Ok(())
    }
    
    #[test]
    fn test_exact_struct_parsing() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
struct SimpleStruct {
    int x;
    int y;
};

struct ComplexStruct {
    int id;
    char name[256];
    SimpleStruct position;
    
    void reset() {
        id = 0;
        position.x = 0;
        position.y = 0;
    }
};

union DataUnion {
    int as_int;
    float as_float;
    char as_bytes[4];
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Check structs
        let structs = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Struct)
            .collect::<Vec<_>>();
        
        assert_eq!(structs.len(), 2, "Should find exactly 2 structs");
        assert!(structs.iter().any(|s| s.name == "SimpleStruct"));
        assert!(structs.iter().any(|s| s.name == "ComplexStruct"));
        
        // Check union
        let unions = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Union)
            .collect::<Vec<_>>();
        
        assert_eq!(unions.len(), 1, "Should find exactly 1 union");
        assert_eq!(unions[0].name, "DataUnion");
        
        // Check fields in ComplexStruct
        let complex_fields = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Field && 
                       s.fqn.contains("ComplexStruct"))
            .collect::<Vec<_>>();
        
        assert!(complex_fields.len() >= 3, "Should find at least 3 fields in ComplexStruct");
        assert!(complex_fields.iter().any(|f| f.name == "id"));
        assert!(complex_fields.iter().any(|f| f.name == "name"));
        assert!(complex_fields.iter().any(|f| f.name == "position"));
        
        // Check method in ComplexStruct
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method &&
                       s.fqn.contains("ComplexStruct"))
            .collect::<Vec<_>>();
        
        assert_eq!(methods.len(), 1, "Should find exactly 1 method in ComplexStruct");
        assert_eq!(methods[0].name, "reset");
        
        Ok(())
    }
}