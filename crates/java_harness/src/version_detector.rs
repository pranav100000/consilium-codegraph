use protocol::{Version, VersionDetection};
use std::path::Path;

/// Detects Java version from various sources
pub struct JavaVersionDetector;

impl JavaVersionDetector {
    /// Detect version from source code features
    pub fn from_source(content: &str) -> VersionDetection {
        // Check for Java 21 features
        if content.contains("Thread.startVirtualThread") || content.contains("virtual Thread") {
            return VersionDetection::new(Version::Java21, 0.9, "Found virtual threads");
        }
        
        // Check for Java 17 features
        if content.contains("sealed ") || content.contains("permits ") {
            return VersionDetection::new(Version::Java17, 0.9, "Found sealed classes");
        }
        if content.contains("case ") && content.contains("->") && !content.contains("switch") {
            return VersionDetection::new(Version::Java17, 0.8, "Found pattern matching in switch");
        }
        if content.contains("\"\"\"") {
            return VersionDetection::new(Version::Java17, 0.8, "Found text blocks");
        }
        
        // Check for Java 11 features  
        if content.contains("var ") && !content.contains("var[") {
            return VersionDetection::new(Version::Java11, 0.8, "Found var keyword");
        }
        if content.contains("HttpClient.newHttpClient") {
            return VersionDetection::new(Version::Java11, 0.8, "Found new HTTP client");
        }
        
        // Check for Java 8 features
        if content.contains("->") && (content.contains("(") || content.contains("::")) {
            return VersionDetection::new(Version::Java8, 0.7, "Found lambda expressions");
        }
        if content.contains("stream()") || content.contains("Stream<") {
            return VersionDetection::new(Version::Java8, 0.7, "Found streams");
        }
        if content.contains("Optional<") || content.contains("Optional.") {
            return VersionDetection::new(Version::Java8, 0.7, "Found Optional");
        }
        if content.contains("@FunctionalInterface") {
            return VersionDetection::new(Version::Java8, 0.8, "Found @FunctionalInterface");
        }
        
        // Default to Java 8 (most common)
        VersionDetection::new(Version::Java8, 0.4, "Default Java version")
    }
    
    /// Detect from build files (pom.xml, build.gradle)
    pub fn from_build_file(content: &str) -> Option<VersionDetection> {
        // Maven pom.xml
        if content.contains("<maven.compiler.source>") {
            if let Some(start) = content.find("<maven.compiler.source>") {
                let rest = &content[start + 23..];
                if let Some(end) = rest.find("</") {
                    let version_str = &rest[..end];
                    return Self::parse_java_version(version_str);
                }
            }
        }
        
        // Gradle build file
        if content.contains("sourceCompatibility") {
            if let Some(start) = content.find("sourceCompatibility") {
                let rest = &content[start..];
                if let Some(eq) = rest.find("=") {
                    let version_part = &rest[eq + 1..];
                    let version_str: String = version_part
                        .chars()
                        .skip_while(|c| c.is_whitespace() || *c == '\'' || *c == '"')
                        .take_while(|c| c.is_numeric() || *c == '.')
                        .collect();
                    return Self::parse_java_version(&version_str);
                }
            }
        }
        
        None
    }
    
    fn parse_java_version(version_str: &str) -> Option<VersionDetection> {
        match version_str {
            "21" | "21.0" => Some(VersionDetection::new(Version::Java21, 0.95, "Found Java 21 in build file")),
            "17" | "17.0" => Some(VersionDetection::new(Version::Java17, 0.95, "Found Java 17 in build file")),
            "11" | "11.0" => Some(VersionDetection::new(Version::Java11, 0.95, "Found Java 11 in build file")),
            "8" | "1.8" | "8.0" => Some(VersionDetection::new(Version::Java8, 0.95, "Found Java 8 in build file")),
            _ => None,
        }
    }
    
    /// Combine multiple detection methods
    pub fn detect(_path: &Path, content: &str) -> Version {
        let mut detections = Vec::new();
        
        // Try build file detection first (highest confidence)
        if let Some(detection) = Self::from_build_file(content) {
            detections.push(detection);
        }
        
        // Try source code analysis
        detections.push(Self::from_source(content));
        
        // Return the detection with highest confidence
        detections.into_iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .map(|d| d.version)
            .unwrap_or(Version::Java8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_java21_detection() {
        let content = r#"
            Thread.startVirtualThread(() -> {
                System.out.println("Virtual thread!");
            });
        "#;
        
        let detection = JavaVersionDetector::from_source(content);
        assert_eq!(detection.version, Version::Java21);
        assert!(detection.confidence > 0.8);
    }
    
    #[test]
    fn test_java17_detection() {
        let content = r#"
            public sealed class Shape permits Circle, Square {}
            
            String text = """
                Multi-line
                text block
                """;
        "#;
        
        let detection = JavaVersionDetector::from_source(content);
        assert_eq!(detection.version, Version::Java17);
    }
    
    #[test]
    fn test_java11_detection() {
        let content = r#"
            var list = new ArrayList<String>();
            var client = HttpClient.newHttpClient();
        "#;
        
        let detection = JavaVersionDetector::from_source(content);
        assert_eq!(detection.version, Version::Java11);
    }
    
    #[test]
    fn test_java8_detection() {
        let content = r#"
            list.stream()
                .filter(s -> s.length() > 5)
                .map(String::toUpperCase)
                .collect(Collectors.toList());
        "#;
        
        let detection = JavaVersionDetector::from_source(content);
        assert_eq!(detection.version, Version::Java8);
    }
    
    #[test]
    fn test_build_file_detection() {
        let pom = r#"
            <properties>
                <maven.compiler.source>17</maven.compiler.source>
                <maven.compiler.target>17</maven.compiler.target>
            </properties>
        "#;
        
        let detection = JavaVersionDetector::from_build_file(pom).unwrap();
        assert_eq!(detection.version, Version::Java17);
        assert!(detection.confidence > 0.9);
    }
}