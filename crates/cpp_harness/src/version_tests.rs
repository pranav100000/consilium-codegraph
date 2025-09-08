#[cfg(test)]
mod version_tests {
    use crate::*;
    use anyhow::Result;
    use protocol::Version;
    
    #[test]
    fn test_version_detection_cpp20() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
#include <coroutine>
#include <concepts>

template<typename T>
concept Numeric = std::is_arithmetic_v<T>;

task<int> compute() {
    co_await something();
    co_return 42;
}

auto cmp = 5 <=> 3;  // spaceship operator
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Check that version was detected
        assert_eq!(harness.version, Some(Version::Cpp20));
        
        // Check that symbols have the version
        for symbol in &symbols {
            assert_eq!(symbol.lang_version, Some(Version::Cpp20));
        }
        
        Ok(())
    }
    
    #[test]
    fn test_version_detection_cpp17() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
#include <optional>

[[nodiscard]] int compute() {
    if constexpr (sizeof(int) == 4) {
        std::optional<int> value = 42;
        return value.value_or(0);
    }
    return 0;
}

auto [x, y] = std::make_pair(1, 2);  // structured binding
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        assert_eq!(harness.version, Some(Version::Cpp17));
        
        for symbol in &symbols {
            assert_eq!(symbol.lang_version, Some(Version::Cpp17));
        }
        
        Ok(())
    }
    
    #[test]
    fn test_version_detection_cpp11() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
#include <memory>

class Widget {
public:
    void process() override final {
        auto ptr = std::make_unique<int>(42);
        auto lambda = [](int x) { return x * 2; };
        nullptr;
    }
    
    Widget() = default;
    Widget(const Widget&) = delete;
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        assert_eq!(harness.version, Some(Version::Cpp11));
        
        Ok(())
    }
    
    #[test]
    fn test_version_detection_c11() -> Result<()> {
        let mut harness = CppHarness::new_c()?;
        let source = r#"
#include <stdalign.h>

_Static_assert(sizeof(int) == 4, "int must be 4 bytes");

struct aligned_struct {
    _Alignas(16) char buffer[256];
};
"#;
        
        let (symbols, _, _) = harness.parse("test.c", source)?;
        
        assert_eq!(harness.version, Some(Version::C11));
        
        Ok(())
    }
    
    #[test]
    fn test_version_from_compiler_flags() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
// Compile with: g++ -std=c++23 -O2 main.cpp

class ModernClass {
    int value;
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        assert_eq!(harness.version, Some(Version::Cpp23));
        
        Ok(())
    }
    
    #[test]
    fn test_explicit_version_setting() -> Result<()> {
        let mut harness = CppHarness::new_with_version(true, Version::Cpp14)?;
        let source = r#"
class Simple {
    int value;
};
"#;
        
        let (symbols, _, _) = harness.parse("test.cpp", source)?;
        
        // Should keep the explicitly set version
        assert_eq!(harness.version, Some(Version::Cpp14));
        
        for symbol in &symbols {
            assert_eq!(symbol.lang_version, Some(Version::Cpp14));
        }
        
        Ok(())
    }
    
    #[test]
    fn test_version_aware_feature_detection() -> Result<()> {
        use protocol::Version;
        
        // Test C++20 features
        assert!(Version::Cpp20.supports_feature("concepts"));
        assert!(Version::Cpp20.supports_feature("coroutines"));
        assert!(Version::Cpp20.supports_feature("modules"));
        assert!(Version::Cpp20.supports_feature("spaceship"));
        
        // Test C++17 features
        assert!(Version::Cpp17.supports_feature("structured_bindings"));
        assert!(Version::Cpp17.supports_feature("if_constexpr"));
        assert!(!Version::Cpp17.supports_feature("concepts"));
        
        // Test C++11 features
        assert!(Version::Cpp11.supports_feature("lambda"));
        assert!(Version::Cpp11.supports_feature("auto"));
        assert!(!Version::Cpp11.supports_feature("structured_bindings"));
        
        // Test that older versions don't support newer features
        assert!(!Version::Cpp98.supports_feature("lambda"));
        assert!(!Version::C99.supports_feature("_Static_assert"));
        
        Ok(())
    }
}