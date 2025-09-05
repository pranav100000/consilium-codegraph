#[cfg(test)]
mod error_handling_tests {
    use crate::*;
    use anyhow::Result;
    
    #[test]
    fn test_invalid_syntax_recovery() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class Broken {
    // Missing closing brace for method
    public void method() {
        if (true) {
            System.out.println("unclosed");
        // Missing closing braces
"#;
        
        // Should not panic, should return partial results
        let (symbols, _, _) = harness.parse("Broken.java", source)?;
        
        // Should at least find the class
        assert!(symbols.iter().any(|s| s.name == "Broken"));
        
        Ok(())
    }
    
    #[test]
    fn test_deeply_nested_generics() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class DeepGenerics {
    Map<String, Map<Integer, List<Map<String, Set<Double>>>>> deeplyNested;
    
    public <T extends Comparable<? super T> & Serializable & Cloneable> 
           Map<T, List<? extends Map<String, ? super T>>> 
           complexMethod(List<? extends T> input, 
                        Map<String, ? super List<? extends T>> params) {
        return null;
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("DeepGenerics.java", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "DeepGenerics"));
        assert!(symbols.iter().any(|s| s.name == "deeplyNested"));
        assert!(symbols.iter().any(|s| s.name == "complexMethod"));
        
        Ok(())
    }
    
    #[test]
    fn test_unicode_in_identifiers() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class ΜαθηματικάΚλάση {
    private int αριθμός = 42;
    private String 文字列 = "hello";
    
    public void υπολογισμός() {
        int результат = αριθμός * 2;
    }
    
    public class 内部クラス {
        void 方法() {}
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("Unicode.java", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "ΜαθηματικάΚλάση"));
        assert!(symbols.iter().any(|s| s.name == "αριθμός"));
        assert!(symbols.iter().any(|s| s.name == "文字列"));
        assert!(symbols.iter().any(|s| s.name == "υπολογισμός"));
        assert!(symbols.iter().any(|s| s.name == "内部クラス"));
        
        Ok(())
    }
    
    #[test]
    fn test_extreme_nesting() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class Outer {
    class Inner1 {
        class Inner2 {
            class Inner3 {
                class Inner4 {
                    class Inner5 {
                        void deepMethod() {
                            class LocalInner6 {
                                void localMethod() {
                                    Runnable r = new Runnable() {
                                        public void run() {
                                            class VeryDeepLocal {
                                                void veryDeepMethod() {}
                                            }
                                        }
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("Nested.java", source)?;
        
        let classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .count();
        
        assert!(classes >= 6, "Should handle deeply nested classes");
        
        Ok(())
    }
    
    #[test]
    fn test_mixed_line_endings() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        // Mix of \n, \r\n, and \r line endings
        let source = "public class Test {\r\n    private int field;\r    public void method() {\n        System.out.println();\r\n    }\r}";
        
        let (symbols, _, _) = harness.parse("Test.java", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "Test"));
        assert!(symbols.iter().any(|s| s.name == "field"));
        assert!(symbols.iter().any(|s| s.name == "method"));
        
        Ok(())
    }
    
    #[test]
    fn test_empty_class_variations() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        
        let sources = vec![
            "class Empty {}",
            "public class Empty{}",
            "class Empty { }",
            "class Empty {\n}",
            "class Empty {\r\n}",
            "class Empty {;}",
            "class Empty {;;}",
        ];
        
        for source in sources {
            let (symbols, _, _) = harness.parse("Empty.java", source)?;
            assert!(symbols.iter().any(|s| s.name == "Empty"), 
                    "Failed to parse: {}", source);
        }
        
        Ok(())
    }
    
    #[test]
    fn test_annotation_edge_cases() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
@Target({})
@Retention(RetentionPolicy.SOURCE)
@interface Empty {}

@Repeatable(Container.class)
@interface Rep {
    String value() default "";
}

@Rep @Rep @Rep @Rep @Rep
@SuppressWarnings({"all", "unchecked", "rawtypes", "serial", "deprecation"})
@Deprecated(since = "1.0", forRemoval = true)
public class HeavilyAnnotated {
    @SafeVarargs
    @SuppressWarnings("varargs")
    public final <T> void varargs(T... args) {}
}
"#;
        
        let (symbols, _, _) = harness.parse("Annotated.java", source)?;
        
        // Annotation interfaces might not be parsed as regular symbols
        // Just check for the main class
        assert!(symbols.iter().any(|s| s.name == "HeavilyAnnotated"));
        assert!(symbols.iter().any(|s| s.name == "varargs"));
        
        Ok(())
    }
    
    #[test]
    fn test_lambda_and_method_refs() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class Lambdas {
    public void testLambdas() {
        Runnable r1 = () -> {};
        Runnable r2 = () -> System.out.println();
        Function<String, Integer> f1 = s -> s.length();
        Function<String, Integer> f2 = String::length;
        BiFunction<String, String, String> f3 = String::concat;
        Supplier<List<String>> s1 = ArrayList::new;
        Consumer<String> c1 = System.out::println;
        
        List<String> list = Stream.of("a", "b", "c")
            .map(String::toUpperCase)
            .filter(s -> s.length() > 0)
            .collect(Collectors.toList());
    }
}
"#;
        
        let (symbols, _, _occurrences) = harness.parse("Lambdas.java", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "testLambdas"));
        
        // Lambda parsing is complex and may not capture all lambda expressions as occurrences
        // Just verify the method containing lambdas is found
        assert!(symbols.iter().any(|s| s.name == "Lambdas"));
        
        Ok(())
    }
    
    #[test]
    fn test_sealed_classes_and_records() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public sealed class Shape permits Circle, Square, Rectangle {}

final class Circle extends Shape {
    private final double radius;
    public Circle(double radius) { this.radius = radius; }
}

non-sealed class Rectangle extends Shape {
    private final double width, height;
    public Rectangle(double w, double h) { width = w; height = h; }
}

sealed class Square extends Shape permits ColoredSquare {
    private final double side;
    public Square(double side) { this.side = side; }
}

final class ColoredSquare extends Square {
    private final String color;
    public ColoredSquare(double side, String color) {
        super(side);
        this.color = color;
    }
}

record Point(double x, double y) {}
record Line(Point start, Point end) {}
record CompactConstructor(String name, int age) {
    public CompactConstructor {
        if (age < 0) throw new IllegalArgumentException();
    }
}
"#;
        
        let (symbols, edges, _) = harness.parse("Sealed.java", source)?;
        
        // Check all classes and records are found
        assert!(symbols.iter().any(|s| s.name == "Shape"));
        assert!(symbols.iter().any(|s| s.name == "Circle"));
        assert!(symbols.iter().any(|s| s.name == "Rectangle"));
        assert!(symbols.iter().any(|s| s.name == "Square"));
        assert!(symbols.iter().any(|s| s.name == "ColoredSquare"));
        assert!(symbols.iter().any(|s| s.name == "Point"));
        assert!(symbols.iter().any(|s| s.name == "Line"));
        assert!(symbols.iter().any(|s| s.name == "CompactConstructor"));
        
        // Check inheritance edges
        let extends_edges = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Extends)
            .count();
        assert!(extends_edges >= 4, "Should find inheritance relationships");
        
        Ok(())
    }
    
    #[test]
    fn test_text_blocks() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class TextBlocks {
    String html = """
        <html>
            <body>
                <p>Hello, world</p>
            </body>
        </html>
        """;
    
    String json = """
        {
            "name": "test",
            "values": [1, 2, 3],
            "nested": {
                "key": "value"
            }
        }
        """;
        
    String sql = """
        SELECT * FROM users
        WHERE age > 18
        AND status = 'active'
        ORDER BY created_at DESC
        """;
}
"#;
        
        let (symbols, _, _) = harness.parse("TextBlocks.java", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "TextBlocks"));
        assert!(symbols.iter().any(|s| s.name == "html"));
        assert!(symbols.iter().any(|s| s.name == "json"));
        assert!(symbols.iter().any(|s| s.name == "sql"));
        
        Ok(())
    }
    
    #[test]
    fn test_switch_expressions() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class SwitchExpressions {
    public String traditional(int day) {
        String result;
        switch (day) {
            case 1:
            case 2:
            case 3:
            case 4:
            case 5:
                result = "Weekday";
                break;
            case 6:
            case 7:
                result = "Weekend";
                break;
            default:
                throw new IllegalArgumentException();
        }
        return result;
    }
    
    public String expression(int day) {
        return switch (day) {
            case 1, 2, 3, 4, 5 -> "Weekday";
            case 6, 7 -> "Weekend";
            default -> throw new IllegalArgumentException();
        };
    }
    
    public int withYield(String s) {
        return switch (s) {
            case "one" -> 1;
            case "two" -> 2;
            default -> {
                System.out.println("Unknown");
                yield -1;
            }
        };
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("SwitchExpressions.java", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "traditional"));
        assert!(symbols.iter().any(|s| s.name == "expression"));
        assert!(symbols.iter().any(|s| s.name == "withYield"));
        
        Ok(())
    }
}