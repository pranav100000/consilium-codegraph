use protocol::{Version, VersionDetection};
use std::path::Path;

/// Detects C/C++ version from various sources
pub struct CppVersionDetector;

impl CppVersionDetector {
    /// Detect version from file extension
    pub fn from_extension(path: &Path) -> Option<VersionDetection> {
        let ext = path.extension()?.to_str()?;
        match ext {
            "c" | "h" => Some(VersionDetection::new(
                Version::C11,  // Default to C11
                0.3,
                "Default C version from .c extension"
            )),
            "cpp" | "cxx" | "cc" | "C" | "c++" => Some(VersionDetection::new(
                Version::Cpp17,  // Default to C++17 (most common)
                0.4,
                "Default C++ version from extension"
            )),
            "hpp" | "hxx" | "h++" => Some(VersionDetection::new(
                Version::Cpp17,
                0.4,
                "Default C++ version from header extension"
            )),
            _ => None,
        }
    }
    
    /// Detect version from source code features
    pub fn from_source(content: &str) -> VersionDetection {
        // Check for C++20 features
        if content.contains("co_await") || content.contains("co_yield") || content.contains("co_return") {
            return VersionDetection::new(Version::Cpp20, 0.95, "Found coroutine keywords");
        }
        if content.contains("concept ") || content.contains("requires ") {
            return VersionDetection::new(Version::Cpp20, 0.9, "Found concepts");
        }
        if content.contains("<=>") {
            return VersionDetection::new(Version::Cpp20, 0.9, "Found spaceship operator");
        }
        if content.contains("import ") && !content.contains("#import") {
            return VersionDetection::new(Version::Cpp20, 0.85, "Found module import");
        }
        
        // Check for C++17 features
        if content.contains("if constexpr") {
            return VersionDetection::new(Version::Cpp17, 0.9, "Found if constexpr");
        }
        if content.contains("[[nodiscard]]") || content.contains("[[maybe_unused]]") {
            return VersionDetection::new(Version::Cpp17, 0.8, "Found C++17 attributes");
        }
        if content.contains("std::optional") || content.contains("std::variant") || content.contains("std::any") {
            return VersionDetection::new(Version::Cpp17, 0.8, "Found C++17 stdlib types");
        }
        
        // Check for C++14 features
        if content.contains("auto ") && content.contains("return") && content.contains("->") {
            return VersionDetection::new(Version::Cpp14, 0.7, "Found auto return type");
        }
        if content.contains("'") && (content.contains("'000") || content.contains("'999")) {
            return VersionDetection::new(Version::Cpp14, 0.6, "Found digit separators");
        }
        
        // Check for C++11 features
        if content.contains("nullptr") {
            return VersionDetection::new(Version::Cpp11, 0.8, "Found nullptr");
        }
        if content.contains("override") || content.contains("final") {
            return VersionDetection::new(Version::Cpp11, 0.8, "Found override/final");
        }
        if content.contains("auto ") && !content.contains("auto*") {
            return VersionDetection::new(Version::Cpp11, 0.7, "Found auto keyword");
        }
        if content.contains("= delete") || content.contains("= default") {
            return VersionDetection::new(Version::Cpp11, 0.8, "Found deleted/defaulted functions");
        }
        if content.contains("constexpr") {
            return VersionDetection::new(Version::Cpp11, 0.8, "Found constexpr");
        }
        if content.contains("std::unique_ptr") || content.contains("std::shared_ptr") {
            return VersionDetection::new(Version::Cpp11, 0.8, "Found smart pointers");
        }
        if content.contains("[]") && (content.contains("](") || content.contains("] (")) {
            return VersionDetection::new(Version::Cpp11, 0.7, "Found lambda expression");
        }
        
        // Check for C features
        if content.contains("_Static_assert") {
            return VersionDetection::new(Version::C11, 0.7, "Found _Static_assert");
        }
        if content.contains("_Alignas") || content.contains("_Alignof") {
            return VersionDetection::new(Version::C11, 0.7, "Found C11 alignment");
        }
        if content.contains("restrict") {
            return VersionDetection::new(Version::C99, 0.6, "Found restrict keyword");
        }
        if content.contains("//") && !content.contains("/*") {
            // Line comments without block comments might indicate C99+
            return VersionDetection::new(Version::C99, 0.3, "Found line comments");
        }
        
        // Check for clear C++ indicators
        if content.contains("class ") || content.contains("namespace ") || content.contains("template<") {
            return VersionDetection::new(Version::Cpp98, 0.6, "Found basic C++ features");
        }
        
        // Default fallback
        if content.contains("#include <iostream>") || content.contains("std::") {
            VersionDetection::new(Version::Cpp98, 0.4, "Found C++ stdlib usage")
        } else {
            VersionDetection::new(Version::C89, 0.3, "No modern features detected")
        }
    }
    
    /// Detect from compiler flags or pragma
    pub fn from_compiler_flags(content: &str) -> Option<VersionDetection> {
        // Check for pragma or comments with version info
        if let Some(pos) = content.find("-std=") {
            let rest = &content[pos + 5..];
            let version_str: String = rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '+')
                .collect();
            
            match version_str.as_str() {
                "c++23" | "c++2b" => Some(VersionDetection::new(Version::Cpp23, 0.95, "Found -std=c++23")),
                "c++20" | "c++2a" => Some(VersionDetection::new(Version::Cpp20, 0.95, "Found -std=c++20")),
                "c++17" | "c++1z" => Some(VersionDetection::new(Version::Cpp17, 0.95, "Found -std=c++17")),
                "c++14" | "c++1y" => Some(VersionDetection::new(Version::Cpp14, 0.95, "Found -std=c++14")),
                "c++11" | "c++0x" => Some(VersionDetection::new(Version::Cpp11, 0.95, "Found -std=c++11")),
                "c++03" => Some(VersionDetection::new(Version::Cpp03, 0.95, "Found -std=c++03")),
                "c++98" => Some(VersionDetection::new(Version::Cpp98, 0.95, "Found -std=c++98")),
                "c23" | "c2x" => Some(VersionDetection::new(Version::C23, 0.95, "Found -std=c23")),
                "c17" | "c18" => Some(VersionDetection::new(Version::C17, 0.95, "Found -std=c17")),
                "c11" => Some(VersionDetection::new(Version::C11, 0.95, "Found -std=c11")),
                "c99" => Some(VersionDetection::new(Version::C99, 0.95, "Found -std=c99")),
                "c89" | "c90" => Some(VersionDetection::new(Version::C89, 0.95, "Found -std=c89")),
                _ => None,
            }
        } else {
            None
        }
    }
    
    /// Combine multiple detection methods
    pub fn detect(path: &Path, content: &str) -> Version {
        let mut detections = Vec::new();
        
        // Try compiler flags first (highest confidence)
        if let Some(detection) = Self::from_compiler_flags(content) {
            detections.push(detection);
        }
        
        // Try source code analysis
        detections.push(Self::from_source(content));
        
        // Try file extension (lowest confidence)
        if let Some(detection) = Self::from_extension(path) {
            detections.push(detection);
        }
        
        // Return the detection with highest confidence
        detections.into_iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .map(|d| d.version)
            .unwrap_or(Version::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cpp20_detection() {
        let content = r#"
            #include <coroutine>
            
            task<int> async_func() {
                co_await something();
                co_return 42;
            }
        "#;
        
        let detection = CppVersionDetector::from_source(content);
        assert_eq!(detection.version, Version::Cpp20);
        assert!(detection.confidence > 0.9);
    }
    
    #[test]
    fn test_cpp17_detection() {
        let content = r#"
            if constexpr (sizeof(int) == 4) {
                [[nodiscard]] auto result = compute();
            }
        "#;
        
        let detection = CppVersionDetector::from_source(content);
        assert_eq!(detection.version, Version::Cpp17);
    }
    
    #[test]
    fn test_cpp11_detection() {
        let content = r#"
            auto ptr = std::make_unique<int>(42);
            void func() override final {
                nullptr;
            }
        "#;
        
        let detection = CppVersionDetector::from_source(content);
        assert_eq!(detection.version, Version::Cpp11);
    }
    
    #[test]
    fn test_compiler_flag_detection() {
        let content = "// Compile with: g++ -std=c++20 -O2 main.cpp";
        
        let detection = CppVersionDetector::from_compiler_flags(content).unwrap();
        assert_eq!(detection.version, Version::Cpp20);
        assert!(detection.confidence > 0.9);
    }
}