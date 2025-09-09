use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a specific version of a programming language
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct LanguageVersion {
    pub language: super::Language,
    pub version: Version,
    pub std_lib: Option<String>,
}

/// Specific version identifiers for each language
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Version {
    // C versions
    C89,
    C99,
    C11,
    C17,
    C23,
    
    // C++ versions
    Cpp98,
    Cpp03,
    Cpp11,
    Cpp14,
    Cpp17,
    Cpp20,
    Cpp23,
    
    // Java versions
    Java8,
    Java11,
    Java17,
    Java21,
    
    // Python versions
    Python2,
    Python3,
    Python38,
    Python39,
    Python310,
    Python311,
    Python312,
    
    // JavaScript/TypeScript versions
    ES5,
    ES6,      // ES2015
    ES2016,
    ES2017,
    ES2018,
    ES2019,
    ES2020,
    ES2021,
    ES2022,
    ES2023,
    
    // Go versions
    Go118,
    Go119,
    Go120,
    Go121,
    
    // .NET versions
    DotNet5,
    DotNet6,
    DotNet7,
    DotNet8,
    DotNetFramework48,
    DotNetCore31,
    
    // Unknown/Auto-detect
    Auto,
    Unknown,
}

impl Version {
    /// Check if this version supports a specific feature
    pub fn supports_feature(&self, feature: &str) -> bool {
        match (self, feature) {
            // C++ features
            (Version::Cpp11 | Version::Cpp14 | Version::Cpp17 | Version::Cpp20 | Version::Cpp23, "auto") => true,
            (Version::Cpp11 | Version::Cpp14 | Version::Cpp17 | Version::Cpp20 | Version::Cpp23, "lambda") => true,
            (Version::Cpp11 | Version::Cpp14 | Version::Cpp17 | Version::Cpp20 | Version::Cpp23, "nullptr") => true,
            (Version::Cpp11 | Version::Cpp14 | Version::Cpp17 | Version::Cpp20 | Version::Cpp23, "override") => true,
            (Version::Cpp14 | Version::Cpp17 | Version::Cpp20 | Version::Cpp23, "auto_return") => true,
            (Version::Cpp17 | Version::Cpp20 | Version::Cpp23, "structured_bindings") => true,
            (Version::Cpp17 | Version::Cpp20 | Version::Cpp23, "if_constexpr") => true,
            (Version::Cpp17 | Version::Cpp20 | Version::Cpp23, "inline_variables") => true,
            (Version::Cpp20 | Version::Cpp23, "concepts") => true,
            (Version::Cpp20 | Version::Cpp23, "coroutines") => true,
            (Version::Cpp20 | Version::Cpp23, "modules") => true,
            (Version::Cpp20 | Version::Cpp23, "ranges") => true,
            (Version::Cpp20 | Version::Cpp23, "spaceship") => true,
            
            // Java features
            (Version::Java8 | Version::Java11 | Version::Java17 | Version::Java21, "lambda") => true,
            (Version::Java8 | Version::Java11 | Version::Java17 | Version::Java21, "stream") => true,
            (Version::Java11 | Version::Java17 | Version::Java21, "var") => true,
            (Version::Java17 | Version::Java21, "sealed") => true,
            (Version::Java17 | Version::Java21, "pattern_matching") => true,
            (Version::Java21, "virtual_threads") => true,
            
            // Python features
            (Version::Python3 | Version::Python38 | Version::Python39 | Version::Python310 | Version::Python311 | Version::Python312, "annotations") => true,
            (Version::Python38 | Version::Python39 | Version::Python310 | Version::Python311 | Version::Python312, "walrus") => true,
            (Version::Python310 | Version::Python311 | Version::Python312, "match") => true,
            (Version::Python311 | Version::Python312, "exception_groups") => true,
            
            // JavaScript/TypeScript features
            (Version::ES6 | Version::ES2016 | Version::ES2017 | Version::ES2018 | Version::ES2019 | Version::ES2020 | Version::ES2021 | Version::ES2022 | Version::ES2023, "arrow_functions") => true,
            (Version::ES6 | Version::ES2016 | Version::ES2017 | Version::ES2018 | Version::ES2019 | Version::ES2020 | Version::ES2021 | Version::ES2022 | Version::ES2023, "let_const") => true,
            (Version::ES6 | Version::ES2016 | Version::ES2017 | Version::ES2018 | Version::ES2019 | Version::ES2020 | Version::ES2021 | Version::ES2022 | Version::ES2023, "template_literals") => true,
            (Version::ES2017 | Version::ES2018 | Version::ES2019 | Version::ES2020 | Version::ES2021 | Version::ES2022 | Version::ES2023, "async_await") => true,
            (Version::ES2020 | Version::ES2021 | Version::ES2022 | Version::ES2023, "optional_chaining") => true,
            (Version::ES2020 | Version::ES2021 | Version::ES2022 | Version::ES2023, "nullish_coalescing") => true,
            
            // Go features
            (Version::Go118 | Version::Go119 | Version::Go120 | Version::Go121, "generics") => true,
            
            _ => false,
        }
    }
    
    /// Get the minimum version that supports a feature
    pub fn minimum_for_feature(language: super::Language, feature: &str) -> Option<Version> {
        match (language, feature) {
            // C++ features
            (super::Language::Cpp, "auto") => Some(Version::Cpp11),
            (super::Language::Cpp, "lambda") => Some(Version::Cpp11),
            (super::Language::Cpp, "structured_bindings") => Some(Version::Cpp17),
            (super::Language::Cpp, "concepts") => Some(Version::Cpp20),
            (super::Language::Cpp, "coroutines") => Some(Version::Cpp20),
            (super::Language::Cpp, "modules") => Some(Version::Cpp20),
            
            // Java features
            (super::Language::Java, "lambda") => Some(Version::Java8),
            (super::Language::Java, "var") => Some(Version::Java11),
            (super::Language::Java, "sealed") => Some(Version::Java17),
            (super::Language::Java, "virtual_threads") => Some(Version::Java21),
            
            // Python features
            (super::Language::Python, "annotations") => Some(Version::Python3),
            (super::Language::Python, "walrus") => Some(Version::Python38),
            (super::Language::Python, "match") => Some(Version::Python310),
            
            // JavaScript/TypeScript features
            (super::Language::TypeScript | super::Language::JavaScript, "arrow_functions") => Some(Version::ES6),
            (super::Language::TypeScript | super::Language::JavaScript, "async_await") => Some(Version::ES2017),
            (super::Language::TypeScript | super::Language::JavaScript, "optional_chaining") => Some(Version::ES2020),
            
            // Go features
            (super::Language::Go, "generics") => Some(Version::Go118),
            
            _ => None,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Version::C89 => "C89",
            Version::C99 => "C99",
            Version::C11 => "C11",
            Version::C17 => "C17",
            Version::C23 => "C23",
            Version::Cpp98 => "C++98",
            Version::Cpp03 => "C++03",
            Version::Cpp11 => "C++11",
            Version::Cpp14 => "C++14",
            Version::Cpp17 => "C++17",
            Version::Cpp20 => "C++20",
            Version::Cpp23 => "C++23",
            Version::Java8 => "Java 8",
            Version::Java11 => "Java 11",
            Version::Java17 => "Java 17",
            Version::Java21 => "Java 21",
            Version::Python2 => "Python 2",
            Version::Python3 => "Python 3",
            Version::Python38 => "Python 3.8",
            Version::Python39 => "Python 3.9",
            Version::Python310 => "Python 3.10",
            Version::Python311 => "Python 3.11",
            Version::Python312 => "Python 3.12",
            Version::ES5 => "ES5",
            Version::ES6 => "ES6/ES2015",
            Version::ES2016 => "ES2016",
            Version::ES2017 => "ES2017",
            Version::ES2018 => "ES2018",
            Version::ES2019 => "ES2019",
            Version::ES2020 => "ES2020",
            Version::ES2021 => "ES2021",
            Version::ES2022 => "ES2022",
            Version::ES2023 => "ES2023",
            Version::Go118 => "Go 1.18",
            Version::Go119 => "Go 1.19",
            Version::Go120 => "Go 1.20",
            Version::Go121 => "Go 1.21",
            Version::DotNet5 => ".NET 5",
            Version::DotNet6 => ".NET 6",
            Version::DotNet7 => ".NET 7",
            Version::DotNet8 => ".NET 8",
            Version::DotNetFramework48 => ".NET Framework 4.8",
            Version::DotNetCore31 => ".NET Core 3.1",
            Version::Auto => "Auto-detect",
            Version::Unknown => "Unknown",
        };
        write!(f, "{}", s)
    }
}

/// Version detection result with confidence
#[derive(Debug, Clone)]
pub struct VersionDetection {
    pub version: Version,
    pub confidence: f32,  // 0.0 to 1.0
    pub reason: String,
}

impl VersionDetection {
    pub fn new(version: Version, confidence: f32, reason: impl Into<String>) -> Self {
        Self {
            version,
            confidence: confidence.min(1.0).max(0.0),
            reason: reason.into(),
        }
    }
}