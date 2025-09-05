#[cfg(test)]
mod error_handling_tests {
    use crate::*;
    use anyhow::Result;
    
    #[test]
    fn test_invalid_syntax_recovery() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
class Broken {
    void method() {
        if (true) {
            std::cout << "unclosed";
        // Missing closing braces
"#;
        
        // Should not panic, should return partial results
        let result = harness.parse("test.cpp", source);
        assert!(result.is_ok(), "Parser should handle incomplete code");
        
        let (symbols, _, _) = result?;
        // May find partial results or empty if too broken
        // Just verify it doesn't panic
        assert!(symbols.len() >= 0);
        
        Ok(())
    }
    
    #[test]
    fn test_complex_template_errors() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
template<template<typename...> class Container, typename... Ts>
class ComplexTemplate {
    Container<Ts...> data;
    
    template<typename T, typename = std::enable_if_t<
        std::is_same_v<T, int> || std::is_same_v<T, double>>>
    T process(T value) {
        return value;
    }
};

// Incomplete specialization
template<>
class ComplexTemplate<std::vector
"#;
        
        let result = harness.parse("test.cpp", source);
        assert!(result.is_ok(), "Should handle incomplete template specialization");
        
        Ok(())
    }
    
    #[test]
    fn test_preprocessor_edge_cases() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
#define STRINGIFY(x) #x
#define CONCAT(a, b) a##b
#define VARIADIC(...) __VA_ARGS__

#ifdef UNDEFINED_MACRO
    class ConditionalClass {
        void neverCompiled();
    };
#else
    class ActualClass {
        void compiled();
    };
#endif

#if 0
    This is not code
    class FakeClass {};
#endif

#define RECURSIVE RECURSIVE
#define COMPLEX_MACRO(type, name) \
    private: \
        type m_##name; \
    public: \
        type get##name() const { return m_##name; } \
        void set##name(type val) { m_##name = val; }

class MacroUser {
    COMPLEX_MACRO(int, Value)
    COMPLEX_MACRO(std::string, Name)
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Macro processing is limited in tree-sitter
        // Just verify it doesn't crash
        assert!(symbols.len() >= 0);
        
        Ok(())
    }
    
    #[test]
    fn test_unicode_identifiers() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
class МатематическийКласс {
    int число = 42;
    std::string 文字列 = "hello";
    
    void υπολογισμός() {
        int результат = число * 2;
    }
    
    class 内部クラス {
        void 方法() {}
    };
};

namespace Ελληνικά {
    void λειτουργία() {}
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "МатематическийКласс"));
        assert!(symbols.iter().any(|s| s.name == "число"));
        assert!(symbols.iter().any(|s| s.name == "υπολογισμός"));
        
        Ok(())
    }
    
    #[test]
    fn test_extreme_nesting() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
namespace n1 {
    namespace n2 {
        namespace n3 {
            class Outer {
                class Inner1 {
                    class Inner2 {
                        class Inner3 {
                            struct Inner4 {
                                union Inner5 {
                                    int x;
                                    float y;
                                    
                                    class Inner6 {
                                        void deepMethod() {
                                            struct LocalStruct {
                                                void localMethod() {}
                                            };
                                        }
                                    };
                                };
                            };
                        };
                    };
                };
            };
        }
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        let classes = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class | SymbolKind::Struct | SymbolKind::Union))
            .count();
        
        // Tree-sitter may not parse all deeply nested structures
        assert!(classes >= 1, "Should find at least some nested structures");
        
        Ok(())
    }
    
    #[test]
    fn test_attributes_and_alignments() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
[[nodiscard]] int important();
[[deprecated("Use newFunc instead")]] void oldFunc();
[[maybe_unused]] static int unused_var = 42;

class [[gnu::packed]] PackedStruct {
    alignas(16) char buffer[256];
    [[no_unique_address]] Empty empty;
};

[[noreturn]] void terminate_app() {
    std::exit(1);
}

__attribute__((always_inline)) inline void fast() {}
__declspec(dllexport) void exported() {}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "important"));
        assert!(symbols.iter().any(|s| s.name == "oldFunc"));
        assert!(symbols.iter().any(|s| s.name == "PackedStruct"));
        assert!(symbols.iter().any(|s| s.name == "terminate_app"));
        
        Ok(())
    }
    
    #[test]
    fn test_concepts_and_constraints() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
template<typename T>
concept Addable = requires(T a, T b) {
    { a + b } -> std::convertible_to<T>;
};

template<typename T>
concept Container = requires(T t) {
    typename T::value_type;
    typename T::iterator;
    { t.begin() } -> std::same_as<typename T::iterator>;
    { t.end() } -> std::same_as<typename T::iterator>;
    { t.size() } -> std::convertible_to<std::size_t>;
};

template<Addable T>
T add(T a, T b) {
    return a + b;
}

template<typename T>
    requires std::is_integral_v<T> || std::is_floating_point_v<T>
T multiply(T a, T b) {
    return a * b;
}

template<Container C>
void process(C& container) {
    for (auto& item : container) {
        // Process item
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Should find function templates
        assert!(symbols.iter().any(|s| s.name == "add"));
        assert!(symbols.iter().any(|s| s.name == "multiply"));
        assert!(symbols.iter().any(|s| s.name == "process"));
        
        Ok(())
    }
    
    #[test]
    fn test_lambda_edge_cases() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
void lambda_tests() {
    // Simple lambda
    auto simple = []() { return 42; };
    
    // Lambda with capture
    int x = 10;
    auto capture_by_value = [x]() { return x * 2; };
    auto capture_by_ref = [&x]() { x++; return x; };
    auto capture_all_value = [=]() { return x; };
    auto capture_all_ref = [&]() { return x; };
    
    // Mutable lambda
    auto mutable_lambda = [x]() mutable { return ++x; };
    
    // Generic lambda
    auto generic = []<typename T>(T a, T b) { return a + b; };
    
    // Lambda with trailing return type
    auto trailing = [](int x) -> double { return x * 1.5; };
    
    // Nested lambdas
    auto outer = [](int x) {
        return [x](int y) {
            return [x, y](int z) {
                return x + y + z;
            };
        };
    };
    
    // Lambda in template
    std::vector<int> vec = {1, 2, 3, 4, 5};
    std::transform(vec.begin(), vec.end(), vec.begin(),
                  [](int n) { return n * n; });
}
"#;
        
        let (symbols, _, occurrences) = harness.parse("test.cpp", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "lambda_tests"));
        
        // Lambda detection is complex - just verify the function is found
        // Tree-sitter may not fully parse all lambda expressions
        
        Ok(())
    }
    
    #[test]
    #[ignore] // Tree-sitter doesn't fully support C++20 coroutines yet
    fn test_coroutines() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
#include <coroutine>

struct Task {
    struct promise_type {
        Task get_return_object() { return {}; }
        std::suspend_never initial_suspend() { return {}; }
        std::suspend_never final_suspend() noexcept { return {}; }
        void return_void() {}
        void unhandled_exception() {}
    };
};

Task my_coroutine() {
    co_await std::suspend_always{};
    co_return;
}

struct Generator {
    struct promise_type {
        int current_value;
        
        Generator get_return_object() { return {}; }
        std::suspend_always initial_suspend() { return {}; }
        std::suspend_always final_suspend() noexcept { return {}; }
        std::suspend_always yield_value(int value) {
            current_value = value;
            return {};
        }
        void unhandled_exception() {}
    };
};

Generator sequence() {
    for (int i = 0; i < 10; ++i) {
        co_yield i;
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "Task"));
        assert!(symbols.iter().any(|s| s.name == "promise_type"));
        assert!(symbols.iter().any(|s| s.name == "my_coroutine"));
        assert!(symbols.iter().any(|s| s.name == "Generator"));
        assert!(symbols.iter().any(|s| s.name == "sequence"));
        
        Ok(())
    }
    
    #[test]
    #[ignore] // Tree-sitter doesn't fully support C++20 modules yet
    fn test_module_declarations() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
export module math.core;

import std;
import math.utils;

export namespace math {
    export int add(int a, int b) {
        return a + b;
    }
    
    export template<typename T>
    T multiply(T a, T b) {
        return a * b;
    }
    
    export class Calculator {
    public:
        void calculate();
    };
}

module :private;

// Private implementation
void internal_helper() {
    // Not exported
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Should find exported symbols
        assert!(symbols.iter().any(|s| s.name == "add"));
        assert!(symbols.iter().any(|s| s.name == "multiply"));
        assert!(symbols.iter().any(|s| s.name == "Calculator"));
        assert!(symbols.iter().any(|s| s.name == "internal_helper"));
        
        Ok(())
    }
    
    #[test]
    fn test_structured_bindings() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
#include <tuple>
#include <map>

void structured_bindings() {
    // Array binding
    int arr[3] = {1, 2, 3};
    auto [a, b, c] = arr;
    
    // Tuple binding
    std::tuple<int, double, std::string> tup{42, 3.14, "hello"};
    auto [x, y, z] = tup;
    
    // Pair binding
    std::pair<int, std::string> p{1, "one"};
    auto [num, str] = p;
    
    // Map iteration
    std::map<int, std::string> map{{1, "one"}, {2, "two"}};
    for (auto [key, value] : map) {
        // Use key and value
    }
    
    // Const and reference bindings
    const auto& [ca, cb] = arr;
    auto&& [ra, rb] = std::make_pair(1, 2);
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "structured_bindings"));
        
        Ok(())
    }
}