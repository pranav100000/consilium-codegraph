#[cfg(test)]
mod stress_tests {
    use crate::*;
    use anyhow::Result;
    
    #[test]
    fn test_extremely_long_identifiers() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let long_name = "A".repeat(10000);
        let source = format!(r#"
class {} {{
    int {}{}{}Member;
    void {}Method() {{}}
}};
"#, long_name, long_name, long_name, long_name, long_name);
        
        // Should handle very long identifiers without stack overflow
        let result = harness.parse("test.cpp", &source);
        assert!(result.is_ok(), "Should handle extremely long identifiers");
        
        Ok(())
    }
    
    #[test]
    fn test_massive_template_instantiations() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let mut source = String::from(r#"
template<typename T> struct A { T value; };
"#);
        
        // Create deeply nested template instantiation
        let mut type_str = String::from("int");
        for i in 0..50 {
            type_str = format!("A<{}>", type_str);
            source.push_str(&format!("{} var{};\n", type_str, i));
        }
        
        let result = harness.parse("test.cpp", &source);
        assert!(result.is_ok());
        
        Ok(())
    }
    
    #[test]
    fn test_preprocessor_chaos() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
// Recursive macros
#define A B
#define B C
#define C D
#define D A

// Token pasting madness
#define PASTE(x, y) x##y
#define PASTE2(x, y) PASTE(x, y)
#define PASTE3(x, y, z) PASTE2(PASTE2(x, y), z)

// Variadic macro abuse
#define VAR(...) __VA_ARGS__
#define VAR2(...) VAR(__VA_ARGS__), VAR(__VA_ARGS__)
#define VAR4(...) VAR2(__VA_ARGS__), VAR2(__VA_ARGS__)

// Macro in macro
#define DECLARE(type) type PASTE(var_, type);
DECLARE(int)
DECLARE(DECLARE(double))

// Conditionals within conditionals
#if defined(UNDEF1)
    #if defined(UNDEF2)
        #if defined(UNDEF3)
            class Never {};
        #elif defined(UNDEF4)
            class NeverEither {};
        #else
            #if 1
                #if 0
                    class Nope {};
                #endif
            #endif
        #endif
    #endif
#else
    class Real {
        VAR4(int, x, y, z)
    };
#endif

// Line continuation abuse
#define LONG_MACRO(a, b, c, d, e, f, g, h) \
    class a { \
        b c; \
        d e; \
        f g; \
        h i; \
    };

LONG_MACRO(MyClass, int, field1, double, field2, char, field3, void*)
"#;
        
        let result = harness.parse("test.cpp", source);
        assert!(result.is_ok());
        
        Ok(())
    }
    
    #[test]
    fn test_template_metaprogramming_madness() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
// SFINAE abuse
template<typename T, typename = void>
struct has_iterator : std::false_type {};

template<typename T>
struct has_iterator<T, std::void_t<typename T::iterator>> : std::true_type {};

// Recursive templates
template<int N>
struct Factorial {
    static constexpr int value = N * Factorial<N-1>::value;
};

template<>
struct Factorial<0> {
    static constexpr int value = 1;
};

// Variadic template recursion
template<typename T, typename... Ts>
struct CountTypes {
    static constexpr size_t value = 1 + CountTypes<Ts...>::value;
};

template<typename T>
struct CountTypes<T> {
    static constexpr size_t value = 1;
};

// Template template parameters
template<template<typename, typename...> class Container, typename T, typename... Args>
class Wrapper {
    Container<T, Args...> data;
    
    template<template<typename> class OtherContainer>
    OtherContainer<T> convert() {
        return OtherContainer<T>{};
    }
};

// Constexpr if
template<typename T>
auto process(T value) {
    if constexpr (std::is_integral_v<T>) {
        return value * 2;
    } else if constexpr (std::is_floating_point_v<T>) {
        return value / 2;
    } else {
        return value;
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        assert!(!symbols.is_empty());
        
        Ok(())
    }
    
    #[test]
    fn test_operator_overloading_everything() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
class OverloadEverything {
public:
    // Arithmetic operators
    OverloadEverything operator+(const OverloadEverything&) const;
    OverloadEverything operator-(const OverloadEverything&) const;
    OverloadEverything operator*(const OverloadEverything&) const;
    OverloadEverything operator/(const OverloadEverything&) const;
    OverloadEverything operator%(const OverloadEverything&) const;
    OverloadEverything operator-() const;
    OverloadEverything& operator++();
    OverloadEverything operator++(int);
    OverloadEverything& operator--();
    OverloadEverything operator--(int);
    
    // Bitwise operators
    OverloadEverything operator&(const OverloadEverything&) const;
    OverloadEverything operator|(const OverloadEverything&) const;
    OverloadEverything operator^(const OverloadEverything&) const;
    OverloadEverything operator~() const;
    OverloadEverything operator<<(int) const;
    OverloadEverything operator>>(int) const;
    
    // Comparison operators
    bool operator==(const OverloadEverything&) const;
    bool operator!=(const OverloadEverything&) const;
    bool operator<(const OverloadEverything&) const;
    bool operator>(const OverloadEverything&) const;
    bool operator<=(const OverloadEverything&) const;
    bool operator>=(const OverloadEverything&) const;
    auto operator<=>(const OverloadEverything&) const = default;
    
    // Logical operators
    bool operator!() const;
    bool operator&&(const OverloadEverything&) const;
    bool operator||(const OverloadEverything&) const;
    
    // Assignment operators
    OverloadEverything& operator=(const OverloadEverything&);
    OverloadEverything& operator+=(const OverloadEverything&);
    OverloadEverything& operator-=(const OverloadEverything&);
    OverloadEverything& operator*=(const OverloadEverything&);
    OverloadEverything& operator/=(const OverloadEverything&);
    OverloadEverything& operator%=(const OverloadEverything&);
    OverloadEverything& operator&=(const OverloadEverything&);
    OverloadEverything& operator|=(const OverloadEverything&);
    OverloadEverything& operator^=(const OverloadEverything&);
    OverloadEverything& operator<<=(int);
    OverloadEverything& operator>>=(int);
    
    // Member access operators
    OverloadEverything& operator*();
    OverloadEverything* operator->();
    OverloadEverything& operator[](size_t);
    
    // Function call operator
    void operator()();
    void operator()(int);
    void operator()(int, double, const char*);
    
    // Type conversion operators
    operator int() const;
    operator double() const;
    operator bool() const;
    explicit operator const char*() const;
    
    // Memory operators
    void* operator new(size_t);
    void* operator new[](size_t);
    void operator delete(void*);
    void operator delete[](void*);
    
    // Comma operator
    OverloadEverything operator,(const OverloadEverything&) const;
    
    // User-defined literals (friend)
    friend OverloadEverything operator""_oe(unsigned long long);
    friend OverloadEverything operator""_oe(long double);
    friend OverloadEverything operator""_oe(const char*, size_t);
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Should find many operator overloads
        let operators = symbols.iter()
            .filter(|s| s.name.starts_with("operator"))
            .count();
        assert!(operators > 20, "Should find many operator overloads");
        
        Ok(())
    }
    
    #[test]
    fn test_massive_inheritance_hierarchy() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let mut source = String::new();
        
        // Create 100 classes in inheritance chain
        for i in 0..100 {
            if i == 0 {
                source.push_str(&format!("class C{} {{ public: virtual void f{} () {{}} }};\n", i, i));
            } else {
                source.push_str(&format!(
                    "class C{} : public C{}, public virtual C0 {{ public: virtual void f{} () override {{}} }};\n", 
                    i, i-1, i
                ));
            }
        }
        
        let result = harness.parse("test.cpp", &source);
        assert!(result.is_ok());
        
        Ok(())
    }
    
    #[test] 
    fn test_union_and_bitfield_chaos() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
union ChaosUnion {
    struct {
        unsigned int bit1 : 1;
        unsigned int bit2 : 2;
        unsigned int bit3 : 3;
        unsigned int bit4 : 4;
        unsigned int bit5 : 5;
        unsigned int bit6 : 6;
        unsigned int bit7 : 7;
        unsigned int : 0;  // Zero-width bitfield
        unsigned int bit8 : 8;
    } bits;
    
    struct {
        char bytes[4];
    } raw;
    
    int as_int;
    float as_float;
    
    struct {
        union {
            int nested_int;
            struct {
                short low;
                short high;
            } parts;
        } nested;
    } complex;
};

struct PackedStruct {
    char c : 8;
    int i : 32;
    long long ll : 64;
    unsigned u : 31;
    bool b : 1;
} __attribute__((packed));
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        assert!(symbols.iter().any(|s| s.name == "ChaosUnion"));
        
        Ok(())
    }
    
    #[test]
    fn test_anonymous_chaos() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
struct Container {
    union {
        struct {
            int x, y;
        };
        struct {
            float fx, fy;
        };
        long long ll;
    };
    
    struct {
        union {
            struct {
                char c1, c2, c3, c4;
            };
            int i;
        };
    } nested;
    
    // Anonymous enum
    enum {
        FLAG1 = 1,
        FLAG2 = 2,
        FLAG3 = 4
    };
    
    // Nested anonymous structures
    struct {
        struct {
            struct {
                int deeply_nested;
            };
        };
    };
};
"#;
        
        let result = harness.parse("test.cpp", source);
        assert!(result.is_ok());
        
        Ok(())
    }
    
    #[test]
    fn test_mixed_c_and_cpp() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
// C-style code
typedef struct {
    int x;
    int y;
} Point;

typedef enum {
    RED,
    GREEN,
    BLUE
} Color;

// Function pointers C-style
typedef int (*func_ptr)(int, int);
func_ptr operations[4];

// C++ style mixed in
class ModernClass {
public:
    using ColorType = Color;
    using PointType = Point;
    
    template<typename T>
    T process(T value) {
        return value;
    }
};

// extern "C" block
extern "C" {
    void c_function(int x);
    
    typedef struct CStruct {
        void (*callback)(void*);
    } CStruct;
}

// Mixing old and new
namespace modern {
    using namespace std;
    
    class X : public ModernClass {
        Point p;
        Color c;
    };
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        assert!(!symbols.is_empty());
        
        Ok(())
    }
    
    #[test]
    fn test_attributes_everywhere() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        // Note: Complex attribute placement can confuse tree-sitter
        // Some functions with attributes in unusual positions may not parse correctly
        let source = r#"
// Standard attribute positions
[[nodiscard]] int normal_func() {
    [[maybe_unused]] int x = 42;
    return x;
}

struct [[gnu::packed]] AlignedStruct {
    alignas(32) char buffer[256];
    [[deprecated]] int old_field;
};

[[noreturn]] void die() {
    throw "death";
}

// Mix of attribute styles
__attribute__((constructor)) void init() {}
__declspec(dllexport) void exported() {}

// Complex attributes on classes  
class [[deprecated("old class")]] OldClass {
public:
    [[nodiscard]] int getValue() const { return 42; }
};
"#;
        
        let result = harness.parse("test.cpp", source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        
        let (symbols, _, _) = result?;
        
        // Should find the functions and structures
        assert!(symbols.iter().any(|s| s.name == "normal_func"));
        assert!(symbols.iter().any(|s| s.name == "AlignedStruct"));
        assert!(symbols.iter().any(|s| s.name == "die"));
        assert!(symbols.iter().any(|s| s.name == "init"));
        assert!(symbols.iter().any(|s| s.name == "exported"));
        assert!(symbols.iter().any(|s| s.name == "OldClass"));
        
        Ok(())
    }
    
    #[test]
    fn test_trigraphs_and_digraphs() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        // Trigraphs are obsolete but test parser resilience
        let source = r#"
// Digraphs (still valid in C++)
class Test <%
    int array<:10:>;
    void method() <%
        array<:0:> = 42;
    %>
%>;

// Alternative tokens
class AltTokens {
    int bit_and = 5 bitand 3;
    int bit_or = 5 bitor 3;
    int bit_xor = 5 xor 3;
    bool bool_and = true and false;
    bool bool_or = true or false;
    bool bool_not = not false;
    
    void test() {
        if (bool_and and_eq true) {
            bit_or or_eq 1;
        }
        if (not bool_not) {
            bit_xor xor_eq 1;
        }
    }
};
"#;
        
        let result = harness.parse("test.cpp", source);
        assert!(result.is_ok());
        
        Ok(())
    }
    
    #[test]
    fn test_deeply_nested_namespaces() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let mut source = String::new();
        
        // Old style deep nesting
        for i in 0..20 {
            source.push_str(&format!("namespace n{} {{\n", i));
        }
        source.push_str("class DeepClass {};\n");
        for _ in 0..20 {
            source.push_str("}\n");
        }
        
        // C++17 style
        source.push_str("namespace a::b::c::d::e::f::g::h::i::j::k::l::m::n::o::p { class ModernDeep {}; }\n");
        
        // Inline namespaces
        source.push_str(r#"
namespace versioned {
    inline namespace v1 {
        inline namespace detail {
            class Implementation {};
        }
    }
}
"#);
        
        let result = harness.parse("test.cpp", &source);
        assert!(result.is_ok());
        
        Ok(())
    }
    
    #[test]
    fn test_complex_string_literals() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r###"
const char* strings[] = {
    "Simple string",
    R"(Raw string)",
    R"delimiter(Raw with )delimiter)delimiter",
    u8"UTF-8 string",
    u"UTF-16 string",
    U"UTF-32 string",
    L"Wide string",
    
    // Concatenation
    "String " "concatenation " "test",
    R"(Raw )" R"(concatenation)",
    
    // Escape sequences
    "Line 1\nLine 2\rLine 3\r\n",
    "Tab\there\tand\tthere",
    "Quote\" and \'apostrophe\'",
    "Null\0character",
    "Hex \x41\x42\x43",
    "Octal \101\102\103",
    "Unicode \u0041\U00000042",
    
    // Very long string
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"
    "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC"
    "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD",
    
    // Empty strings
    "",
    R"()",
    u8"",
    
    // Special characters (would contain unicode in real code)
    "emoji_here", // Emoji
    "arabic_text", // Arabic
    "chinese_text", // Chinese
    "japanese_text", // Japanese
};
"###;
        
        let result = harness.parse("test.cpp", source);
        assert!(result.is_ok());
        
        Ok(())
    }
}