#[cfg(test)]
mod complex_tests {
    use crate::*;
    use anyhow::Result;
    
    #[test]
    fn test_deeply_nested_namespaces() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
namespace level1 {
    namespace level2 {
        namespace level3 {
            class DeepClass {
                void method() {
                    std::cout << "Deep!" << std::endl;
                }
            };
        }
    }
}

namespace level1::level2::level3 {
    class AnotherDeep {
        void another() {}
    };
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        let deep_class = symbols.iter()
            .find(|s| s.name == "DeepClass");
        assert!(deep_class.is_some());
        assert!(deep_class.unwrap().fqn.contains("level1::level2::level3"));
        
        let another = symbols.iter()
            .find(|s| s.name == "AnotherDeep");
        assert!(another.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_complex_template_instantiation() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
template<typename T, typename U = std::string>
class Pair {
    T first;
    U second;
public:
    Pair(T t, U u) : first(t), second(u) {}
};

template<>
class Pair<int, int> {
    int sum() { return 0; }
};

template<typename... Args>
void variadic(Args... args) {}

template<template<typename> class Container, typename T>
class Wrapper {
    Container<T> data;
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        let pair = symbols.iter().find(|s| s.name == "Pair");
        assert!(pair.is_some());
        
        let variadic = symbols.iter().find(|s| s.name == "variadic");
        assert!(variadic.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_complex_inheritance_chain() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
class A {
    virtual void method() = 0;
};

class B : public A {
    void method() override {}
};

class C : public B {
    void method() override final {}
};

class D : public C, private std::enable_shared_from_this<D> {
    void another() {}
};

class Diamond1 {
    virtual void foo() {}
};

class Diamond2 {
    virtual void bar() {}
};

class MultiInherit : public Diamond1, public Diamond2 {
    void foo() override {}
    void bar() override {}
};
"#;
        
        let (symbols, edges, _) = harness.parse("test.cpp", source)?;
        
        let extends_edges = edges.iter()
            .filter(|e| e.edge_type == protocol::EdgeType::Extends)
            .count();
        
        assert!(extends_edges >= 5, "Should find multiple inheritance edges");
        
        Ok(())
    }
    
    #[test]
    fn test_macro_usage_and_preprocessing() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
#define MAX(a,b) ((a) > (b) ? (a) : (b))
#define CONCAT(x, y) x##y
#define STRINGIFY(x) #x

#ifdef DEBUG
    #define LOG(msg) std::cout << msg << std::endl
#else
    #define LOG(msg)
#endif

class MacroUser {
    void test() {
        int x = MAX(5, 10);
        LOG("Testing");
        auto var = CONCAT(test, 123);
    }
};

#define CLASS_DECL(name) \
    class name { \
    public: \
        void method(); \
    };

CLASS_DECL(Generated)
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        let macro_user = symbols.iter().find(|s| s.name == "MacroUser");
        assert!(macro_user.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_complex_function_pointers() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
typedef int (*SimpleFunc)(int);
typedef void (*ComplexFunc)(int, const char*, ...);

using ModernFunc = std::function<void(int)>;
using CallbackFunc = void(*)(void* userData, int status);

class CallbackManager {
    std::vector<CallbackFunc> callbacks;
    void (*errorHandler)(const std::string&);
    
    void registerCallback(CallbackFunc cb) {
        callbacks.push_back(cb);
    }
    
    int process(int (*transformer)(int)) {
        return transformer(42);
    }
};

int transform(int x) { return x * 2; }

void usage() {
    CallbackManager mgr;
    mgr.process(&transform);
    mgr.process([](int x) { return x + 1; });
}
"#;
        
        let (symbols, edges, _) = harness.parse("test.cpp", source)?;
        
        let callback_mgr = symbols.iter().find(|s| s.name == "CallbackManager");
        assert!(callback_mgr.is_some());
        
        let register_cb = symbols.iter().find(|s| s.name == "registerCallback");
        assert!(register_cb.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_constexpr_and_concepts() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
template<typename T>
concept Numeric = std::is_arithmetic_v<T>;

template<Numeric T>
constexpr T square(T x) {
    return x * x;
}

constexpr int factorial(int n) {
    return n <= 1 ? 1 : n * factorial(n - 1);
}

template<int N>
struct Fibonacci {
    static constexpr int value = Fibonacci<N-1>::value + Fibonacci<N-2>::value;
};

template<>
struct Fibonacci<0> {
    static constexpr int value = 0;
};

template<>
struct Fibonacci<1> {
    static constexpr int value = 1;
};

consteval int compile_time_only() {
    return 42;
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        let square = symbols.iter().find(|s| s.name == "square");
        assert!(square.is_some());
        
        let factorial = symbols.iter().find(|s| s.name == "factorial");
        assert!(factorial.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_friend_declarations() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
class SecretKeeper {
private:
    int secret = 42;
    friend class TrustedFriend;
    friend void accessSecret(const SecretKeeper&);
    
    template<typename T>
    friend class TemplatedFriend;
};

class TrustedFriend {
    void peek(const SecretKeeper& sk) {
        int val = sk.secret;
    }
};

void accessSecret(const SecretKeeper& sk) {
    int val = sk.secret;
}

template<typename T>
class TemplatedFriend {
    void access(const SecretKeeper& sk) {
        int val = sk.secret;
    }
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        let secret_keeper = symbols.iter().find(|s| s.name == "SecretKeeper");
        assert!(secret_keeper.is_some());
        
        let trusted = symbols.iter().find(|s| s.name == "TrustedFriend");
        assert!(trusted.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_complex_initializer_lists() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
class Complex {
    int a, b, c;
    std::vector<int> vec;
    std::map<std::string, int> map;
    
public:
    Complex() : a(1), b(2), c(3), 
                vec{1, 2, 3, 4, 5},
                map{{"one", 1}, {"two", 2}} {}
    
    Complex(int x) : Complex() {
        a = x;
    }
    
    Complex(std::initializer_list<int> init) : vec(init) {}
};

void test() {
    Complex c1;
    Complex c2(10);
    Complex c3{1, 2, 3, 4};
    
    std::vector<std::vector<int>> matrix = {
        {1, 2, 3},
        {4, 5, 6},
        {7, 8, 9}
    };
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        let complex = symbols.iter().find(|s| s.name == "Complex");
        assert!(complex.is_some());
        
        // Check for the test method which should definitely be parsed
        let test_method = symbols.iter().find(|s| s.name == "test");
        assert!(test_method.is_some(), "Should find the test method");
        
        Ok(())
    }
    
    #[test]
    fn test_attributes_and_alignments() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
[[nodiscard]] int important_result() {
    return 42;
}

[[deprecated("Use new_function instead")]]
void old_function() {}

class [[gnu::packed]] PackedStruct {
    char a;
    int b;
    char c;
};

struct alignas(16) AlignedData {
    float data[4];
};

[[noreturn]] void terminate_app() {
    std::exit(1);
}

[[maybe_unused]] static int debug_counter = 0;

namespace [[deprecated]] old_api {
    void legacy_function() {}
}
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        let important = symbols.iter().find(|s| s.name == "important_result");
        assert!(important.is_some());
        
        let packed = symbols.iter().find(|s| s.name == "PackedStruct");
        assert!(packed.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_complex_using_declarations() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
namespace ns1 {
    void func1() {}
    void func2() {}
    
    namespace inner {
        class InnerClass {};
    }
}

namespace ns2 {
    using ns1::func1;
    using namespace ns1::inner;
    
    class Derived : public InnerClass {
        using InnerClass::InnerClass;
    };
}

template<typename T>
class Base {
protected:
    void method() {}
};

template<typename T>
class Derived : public Base<T> {
public:
    using Base<T>::method;
    using typename Base<T>::value_type;
};

using IntVector = std::vector<int>;
template<typename T>
using SharedPtr = std::shared_ptr<T>;
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        let derived = symbols.iter()
            .filter(|s| s.name == "Derived")
            .count();
        
        assert!(derived >= 1);
        
        Ok(())
    }
}