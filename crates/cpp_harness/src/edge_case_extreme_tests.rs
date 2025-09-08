#[cfg(test)]
mod edge_case_extreme_tests {
    use crate::*;
    use anyhow::Result;
    use protocol::Version;
    
    #[test]
    fn test_null_bytes_in_strings() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
const char* data = "Hello\0World";
char buffer[] = "Test\x00Data";
std::string s = "Null\000Byte";
"#;
        
        let result = harness.parse("test.cpp", source);
        assert!(result.is_ok(), "Should handle null bytes in strings");
        
        Ok(())
    }
    
    #[test]
    fn test_maximum_template_nesting() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        
        // Generate deeply nested templates
        let mut source = String::from("template<");
        for i in 0..50 {
            source.push_str(&format!("typename T{}, ", i));
        }
        source.push_str("typename TLast>\nclass DeepTemplate {};\n");
        
        let result = harness.parse("test.cpp", &source);
        assert!(result.is_ok(), "Should handle deep template nesting");
        
        Ok(())
    }
    
    #[test]
    fn test_recursive_macro_expansion() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
#define A B
#define B C
#define C D
#define D E
#define E F
#define F G
#define G H
#define H "final"

const char* result = A;  // Should expand to "final"
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // We should at least find the macros and the variable
        assert!(symbols.iter().any(|s| s.name == "result"));
        
        Ok(())
    }
    
    #[test]
    fn test_conflicting_version_features() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
// Uses both old and new features
auto lambda = []() { return 42; };  // C++11
int array[] = {1, 2, 3};  // C++98
auto [a, b, c] = std::make_tuple(1, 2, 3);  // C++17
typedef int INT;  // C style
using FLOAT = float;  // C++11
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Version should be detected as at least C++11 (structured bindings require C++17 but might not be detected)
        assert!(matches!(harness.version, Some(Version::Cpp11) | Some(Version::Cpp14) | Some(Version::Cpp17)));
        
        Ok(())
    }
    
    #[test]
    fn test_incomplete_class_forward_references() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
class A;  // Forward declaration
class B;
class C;

class A {
    B* b;
    C* c;
};

class B {
    A* a;
    C* c;
};

class C {
    A* a;
    B* b;
};

// Circular dependency through pointers
"#;
        
        let (symbols, edges, _) = harness.parse("test.cpp", source)?;
        
        // Should find both forward declarations and definitions
        let class_count = symbols.iter().filter(|s| s.kind == SymbolKind::Class).count();
        assert!(class_count >= 3, "Should find at least 3 classes, found: {}", class_count);
        
        Ok(())
    }
    
    #[test]
    fn test_mixed_encoding_comments() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
// English comment
// 中文注释
// コメント
// Комментарий
// تعليق
/* Multi-line
   with mixed
   编码 encoding */
class MixedEncodingClass {
    int 数字 = 42;  // Chinese identifier
    int число = 24;  // Russian identifier
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "MixedEncodingClass"));
        
        Ok(())
    }
    
    #[test]
    fn test_operator_overloading_all_operators() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
class OperatorMadness {
public:
    // Arithmetic
    OperatorMadness operator+(const OperatorMadness&) const;
    OperatorMadness operator-(const OperatorMadness&) const;
    OperatorMadness operator*(const OperatorMadness&) const;
    OperatorMadness operator/(const OperatorMadness&) const;
    OperatorMadness operator%(const OperatorMadness&) const;
    
    // Bitwise
    OperatorMadness operator&(const OperatorMadness&) const;
    OperatorMadness operator|(const OperatorMadness&) const;
    OperatorMadness operator^(const OperatorMadness&) const;
    OperatorMadness operator~() const;
    OperatorMadness operator<<(int) const;
    OperatorMadness operator>>(int) const;
    
    // Logical
    bool operator&&(const OperatorMadness&) const;
    bool operator||(const OperatorMadness&) const;
    bool operator!() const;
    
    // Comparison
    bool operator==(const OperatorMadness&) const;
    bool operator!=(const OperatorMadness&) const;
    bool operator<(const OperatorMadness&) const;
    bool operator>(const OperatorMadness&) const;
    bool operator<=(const OperatorMadness&) const;
    bool operator>=(const OperatorMadness&) const;
    auto operator<=>(const OperatorMadness&) const;  // C++20
    
    // Assignment
    OperatorMadness& operator=(const OperatorMadness&);
    OperatorMadness& operator+=(const OperatorMadness&);
    OperatorMadness& operator-=(const OperatorMadness&);
    OperatorMadness& operator*=(const OperatorMadness&);
    OperatorMadness& operator/=(const OperatorMadness&);
    OperatorMadness& operator%=(const OperatorMadness&);
    OperatorMadness& operator&=(const OperatorMadness&);
    OperatorMadness& operator|=(const OperatorMadness&);
    OperatorMadness& operator^=(const OperatorMadness&);
    OperatorMadness& operator<<=(int);
    OperatorMadness& operator>>=(int);
    
    // Increment/Decrement
    OperatorMadness& operator++();     // Prefix
    OperatorMadness operator++(int);   // Postfix
    OperatorMadness& operator--();     // Prefix
    OperatorMadness operator--(int);   // Postfix
    
    // Member access
    OperatorMadness* operator->();
    OperatorMadness& operator*();
    
    // Function call
    void operator()();
    void operator()(int);
    void operator()(int, double);
    
    // Subscript
    int& operator[](int);
    const int& operator[](int) const;
    
    // Type conversion
    operator bool() const;
    operator int() const;
    explicit operator double() const;
    
    // Memory management
    void* operator new(size_t);
    void* operator new[](size_t);
    void operator delete(void*);
    void operator delete[](void*);
    
    // Comma
    OperatorMadness operator,(const OperatorMadness&);
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Should find all operator overloads
        let operators = symbols.iter()
            .filter(|s| s.name.starts_with("operator"))
            .count();
        
        assert!(operators > 40, "Should find many operator overloads, found: {}", operators);
        
        Ok(())
    }
    
    #[test]
    fn test_preprocessor_madness() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
#define PASTE(a, b) a##b
#define STRINGIFY(x) #x
#define EXPAND_STRINGIFY(x) STRINGIFY(x)

#define VERSION 123
const char* version_str = EXPAND_STRINGIFY(VERSION);  // "123"

#define DECLARE_FUNC(type, name) \
    type PASTE(get_, name)(); \
    void PASTE(set_, name)(type value);

DECLARE_FUNC(int, value)
DECLARE_FUNC(double, ratio)

#if defined(__cplusplus) && __cplusplus >= 201703L
    #define CPP17_FEATURE
#elif defined(__cplusplus) && __cplusplus >= 201402L
    #define CPP14_FEATURE
#else
    #error "Unsupported C++ version"
#endif

#ifdef NEVER_DEFINED
    class NeverCompiled {
        void invisible();
    };
#endif

#ifndef GUARD_H
#define GUARD_H
    class GuardedClass {};
#endif

#pragma once
#pragma pack(push, 1)
struct PackedStruct {
    char a;
    int b;
};
#pragma pack(pop)

#line 1000 "fake_file.cpp"
void fake_location() {}  // Should appear to be from line 1000
"#;
        
        let result = harness.parse("test.cpp", source);
        assert!(result.is_ok(), "Should handle complex preprocessor directives");
        
        Ok(())
    }
    
    #[test]
    fn test_anonymous_unions_and_structs() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
struct Container {
    union {  // Anonymous union
        int i;
        float f;
        char c[4];
    };
    
    struct {  // Anonymous struct
        double x;
        double y;
        double z;
    };
    
    union {
        struct {
            uint16_t low;
            uint16_t high;
        };
        uint32_t full;
    };
};

union OuterUnion {
    struct {
        int a, b, c;
    } s;
    
    int array[3];
    
    struct {  // Another anonymous struct
        char bytes[12];
    };
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "Container"));
        assert!(symbols.iter().any(|s| s.name == "OuterUnion"));
        
        Ok(())
    }
    
    #[test]
    fn test_extreme_inheritance_depth() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        
        let mut source = String::new();
        
        // Create 100 levels of inheritance
        for i in 0..100 {
            if i == 0 {
                source.push_str("class Level0 { int value; };\n");
            } else {
                source.push_str(&format!("class Level{} : public Level{} {{}};\n", i, i-1));
            }
        }
        
        let (symbols, edges, _) = harness.parse("test.cpp", &source)?;
        
        assert_eq!(symbols.iter().filter(|s| s.kind == SymbolKind::Class).count(), 100);
        assert!(edges.iter().any(|e| e.edge_type == EdgeType::Extends));
        
        Ok(())
    }
    
    #[test]
    fn test_variadic_everything() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
// Variadic function
void printf_like(const char* fmt, ...);

// Variadic template
template<typename... Args>
void variadic_template(Args... args) {
    ((std::cout << args << " "), ...);  // C++17 fold expression
}

// Variadic macro
#define LOG(fmt, ...) printf(fmt, __VA_ARGS__)
#define LOG2(fmt, ...) printf(fmt, ##__VA_ARGS__)  // GNU extension

// Recursive variadic template
template<typename T>
void print(T&& t) {
    std::cout << t << std::endl;
}

template<typename T, typename... Args>
void print(T&& t, Args&&... args) {
    std::cout << t << " ";
    print(args...);
}

// Parameter pack expansion
template<typename... Types>
class Tuple {};

template<typename... Types>
void forward_all(Types&&... args) {
    other_function(std::forward<Types>(args)...);
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "printf_like"));
        assert!(symbols.iter().any(|s| s.name == "variadic_template"));
        
        Ok(())
    }
    
    #[test]
    fn test_gnu_extensions() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
// GNU statement expressions
int x = ({ int a = 5; a * 2; });

// GNU typeof
typeof(x) y = 10;

// GNU case ranges
switch (value) {
    case 1 ... 10:
        break;
    case 'a' ... 'z':
        break;
}

// GNU nested functions (in C)
void outer() {
    void inner() {
        printf("Nested function\n");
    }
    inner();
}

// GNU designated initializers
struct Point p = { .x = 1, .y = 2 };
int arr[] = { [0] = 1, [5] = 2, [3] = 3 };

// GNU __attribute__
__attribute__((packed)) struct PackedStruct {
    char c;
    int i;
};

__attribute__((always_inline)) inline void fast() {}
__attribute__((noreturn)) void die();
__attribute__((constructor)) void init();
__attribute__((destructor)) void cleanup();
"#;
        
        let result = harness.parse("test.cpp", source);
        assert!(result.is_ok(), "Should handle GNU extensions");
        
        Ok(())
    }
    
    #[test]
    fn test_concepts_and_requires() -> Result<()> {
        let mut harness = CppHarness::new_with_version(true, Version::Cpp20)?;
        let source = r#"
// Concepts
template<typename T>
concept Integral = std::is_integral_v<T>;

template<typename T>
concept SignedIntegral = Integral<T> && std::is_signed_v<T>;

template<typename T>
concept Addable = requires(T a, T b) {
    { a + b } -> std::convertible_to<T>;
};

template<typename T>
concept Complex = requires(T t) {
    typename T::value_type;
    { t.real() } -> std::convertible_to<double>;
    { t.imag() } -> std::convertible_to<double>;
};

// Requires clauses
template<typename T>
    requires Integral<T>
T add(T a, T b) {
    return a + b;
}

template<typename T>
    requires Integral<T> && sizeof(T) >= 4
T multiply(T a, T b) {
    return a * b;
}

// Constrained auto
Integral auto x = 42;
SignedIntegral auto y = -10;

// Abbreviated function templates
void process(Integral auto value) {
    std::cout << value;
}

// Compound requirements
template<typename T>
concept Stringable = requires(T t) {
    { t.toString() } -> std::convertible_to<std::string>;
    { t.toString() } noexcept;
    requires sizeof(t) <= 100;
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Should find concepts and constrained templates
        assert!(symbols.iter().any(|s| s.name == "add"));
        assert!(symbols.iter().any(|s| s.name == "multiply"));
        
        Ok(())
    }
    
    #[test]
    fn test_modules_and_export() -> Result<()> {
        let mut harness = CppHarness::new_with_version(true, Version::Cpp20)?;
        let source = r#"
// Module declaration
export module math.core;

import std;
import math.utils;

// Export declarations
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

export namespace math {
    const double PI = 3.14159;
    
    export double area(double radius) {
        return PI * radius * radius;
    }
}

// Module partition
export module math:advanced;

// Module implementation unit
module math;

void internal_helper() {
    // Not exported
}

// Global module fragment
module;
#include <cstdlib>
"#;
        
        let result = harness.parse("test.cpp", source);
        // Tree-sitter might not fully support modules yet
        assert!(result.is_ok(), "Should at least not crash on module syntax");
        
        Ok(())
    }
    
    #[test]
    fn test_coroutines() -> Result<()> {
        let mut harness = CppHarness::new_with_version(true, Version::Cpp20)?;
        let source = r#"
#include <coroutine>

template<typename T>
struct Generator {
    struct promise_type {
        T current_value;
        
        Generator get_return_object() {
            return Generator{std::coroutine_handle<promise_type>::from_promise(*this)};
        }
        
        std::suspend_always initial_suspend() { return {}; }
        std::suspend_always final_suspend() noexcept { return {}; }
        
        std::suspend_always yield_value(T value) {
            current_value = value;
            return {};
        }
        
        void unhandled_exception() {}
        void return_void() {}
    };
    
    std::coroutine_handle<promise_type> coro;
};

Generator<int> fibonacci() {
    int a = 0, b = 1;
    while (true) {
        co_yield a;
        auto tmp = a;
        a = b;
        b = tmp + b;
    }
}

Task async_operation() {
    co_await some_async_call();
    co_return 42;
}
"#;
        
        let result = harness.parse("test.cpp", source);
        assert!(result.is_ok(), "Should handle coroutine syntax");
        
        Ok(())
    }
    
    #[test]
    fn test_sfinae_and_enable_if() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
// SFINAE with enable_if
template<typename T, 
         typename = std::enable_if_t<std::is_integral_v<T>>>
T increment(T value) {
    return value + 1;
}

// SFINAE with decltype
template<typename T>
auto has_size(T&& t) -> decltype(t.size(), std::true_type{}) {
    return std::true_type{};
}

std::false_type has_size(...) {
    return std::false_type{};
}

// Expression SFINAE
template<typename T>
class has_iterator {
    template<typename U>
    static auto test(U*) -> decltype(
        std::declval<U>().begin(),
        std::declval<U>().end(),
        std::true_type{}
    );
    
    template<typename>
    static std::false_type test(...);
    
public:
    static constexpr bool value = decltype(test<T>(nullptr))::value;
};

// Trailing return type with decltype
template<typename T, typename U>
auto add(T t, U u) -> decltype(t + u) {
    return t + u;
}

// if constexpr (C++17)
template<typename T>
void process(T value) {
    if constexpr (std::is_integral_v<T>) {
        std::cout << "Integer: " << value;
    } else if constexpr (std::is_floating_point_v<T>) {
        std::cout << "Float: " << value;
    } else {
        std::cout << "Other: " << value;
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "increment"));
        assert!(symbols.iter().any(|s| s.name == "has_size"));
        assert!(symbols.iter().any(|s| s.name == "process"));
        
        Ok(())
    }
}