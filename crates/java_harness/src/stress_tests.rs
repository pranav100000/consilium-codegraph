#[cfg(test)]
mod stress_tests {
    use crate::*;
    use anyhow::Result;
    
    #[test]
    fn test_extremely_long_identifiers() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let long_name = "A".repeat(10000);
        let source = format!(r#"
public class {} {{
    private int {}{}{}Field;
    public void {}Method() {{}}
}}
"#, long_name, long_name, long_name, long_name, long_name);
        
        // Should handle very long identifiers without stack overflow
        let result = harness.parse("LongNames.java", &source);
        assert!(result.is_ok(), "Should handle extremely long identifiers");
        
        Ok(())
    }
    
    #[test]
    fn test_massive_class_with_thousands_of_members() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let mut source = String::from("public class MassiveClass {\n");
        
        // Add 1000 fields
        for i in 0..1000 {
            source.push_str(&format!("    private int field{};\n", i));
        }
        
        // Add 1000 methods
        for i in 0..1000 {
            source.push_str(&format!("    public void method{}() {{}}\n", i));
        }
        
        source.push_str("}\n");
        
        let (symbols, _, _) = harness.parse("Massive.java", &source)?;
        
        // Should handle thousands of members
        assert!(symbols.len() >= 2000, "Should parse all members");
        
        Ok(())
    }
    
    #[test]
    fn test_deeply_nested_blocks() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let mut source = String::from("public class DeepNest { void m() {");
        
        // Create 100 levels of nested blocks
        for _ in 0..100 {
            source.push_str("{ if (true) { try { for(;;) { while(true) {");
        }
        
        source.push_str("int x = 1;");
        
        for _ in 0..100 {
            source.push_str("}}}}}");
        }
        
        source.push_str("}}");
        
        // Should handle deep nesting without stack overflow
        let result = harness.parse("DeepNest.java", &source);
        assert!(result.is_ok(), "Should handle deeply nested blocks");
        
        Ok(())
    }
    
    #[test]
    fn test_malformed_unicode_and_control_characters() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        
        // Test with various control characters and malformed unicode
        let sources = vec![
            "class \u{0000}Test {}", // Null character
            "class \u{0001}Test {}", // Control character
            "class Test\u{FEFF} {}", // Zero-width no-break space
            "class Test\u{200B} {}", // Zero-width space
            "class \u{202E}tseT {}", // Right-to-left override
            "class Test\r\n\r\n\r {}", // Mixed line endings
            "class /* \u{0000} */ Test {}", // Null in comment
            "class Test { String s = \"\u{0000}\"; }", // Null in string
        ];
        
        for source in sources {
            let result = harness.parse("Unicode.java", source);
            assert!(result.is_ok(), "Should handle malformed unicode: {}", source);
        }
        
        Ok(())
    }
    
    #[test]
    fn test_pathological_generics() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        
        // Create extremely complex nested generics
        let source = r#"
public class PathologicalGenerics<
    T extends Comparable<? super T> & Serializable,
    U extends Map<String, List<Map<Integer, Set<Map<String, List<Double>>>>>>,
    V extends Function<
        Map<String, List<Set<Map<Integer, String>>>>,
        List<Map<Set<String>, Map<Integer, List<Set<Double>>>>>
    >
> {
    Map<
        Map<String, Map<Integer, Map<Double, Map<Float, String>>>>,
        Map<List<Set<Map<Integer, String>>>, Map<String, List<Double>>>
    > deeplyNestedField;
    
    public <
        A extends Map<?, List<? extends Set<? super Map<String, ?>>>>,
        B extends Comparable<? super B> & Serializable & Cloneable,
        C extends Map<String, ? extends List<? super Set<? extends Map<?, ?>>>>
    > Map<A, Map<B, C>> complexMethod(
        List<? extends Map<String, ? super List<? extends Set<? super A>>>> param1,
        Map<? super B, ? extends List<? super Set<? extends C>>> param2
    ) {
        return null;
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("Pathological.java", source)?;
        assert!(symbols.iter().any(|s| s.name == "PathologicalGenerics"));
        
        Ok(())
    }
    
    #[test]
    fn test_massive_string_literals() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        
        // Create huge string literals
        let huge_string = "x".repeat(100000);
        let source = format!(r#"
public class HugeStrings {{
    String s1 = "{}";
    String s2 = """
                {}
                """;
    char[] c = {{{}}};
}}
"#, huge_string, huge_string, 
    (0..1000).map(|_| "'x'").collect::<Vec<_>>().join(","));
        
        let result = harness.parse("HugeStrings.java", &source);
        assert!(result.is_ok(), "Should handle massive string literals");
        
        Ok(())
    }
    
    #[test]
    fn test_circular_reference_patterns() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
class A extends B {}
class B extends C {}
class C implements D {}
interface D extends E {}
interface E extends F {}
interface F {}

class Outer {
    class Inner1 extends Outer.Inner2 {}
    class Inner2 extends Outer.Inner1 {}
}

interface I1 extends I2, I3 {}
interface I2 extends I3, I4 {}
interface I3 extends I4, I1 {}
interface I4 extends I1, I2 {}
"#;
        
        // Should handle circular references without infinite loops
        let (symbols, edges, _) = harness.parse("Circular.java", source)?;
        assert!(!symbols.is_empty());
        assert!(!edges.is_empty());
        
        Ok(())
    }
    
    #[test]
    fn test_comment_bombs() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        
        // Test various comment edge cases
        let source = r#"
// /* Mixed comment */ still line comment
/* // Nested? /* Really? */ */
/** /* /** Nested javadoc? */ */
/* Unterminated comment...
class Test {
    /* Another unterminated...
}
/* Multi
 * line
 * with
 * many
 * stars
 * * * * *
 ****************/
class Real {}
"#;
        
        let result = harness.parse("Comments.java", source);
        assert!(result.is_ok());
        
        Ok(())
    }
    
    #[test]
    fn test_annotation_bombs() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        
        // Create deeply nested and repeated annotations
        let mut source = String::new();
        
        // Add 100 annotations to a single class
        for i in 0..100 {
            source.push_str(&format!("@Annotation{}\n", i));
        }
        
        source.push_str(r#"
@ComplexAnnotation(
    value = @Nested(@Nested(@Nested(@Nested(@Nested("deep"))))),
    array = {@One, @Two, @Three, @Four, @Five, @Six, @Seven, @Eight, @Nine, @Ten},
    types = {String.class, Integer.class, Double.class, Float.class}
)
public class AnnotationBomb {
    "#);
        
        // Add 100 annotated fields
        for i in 0..100 {
            source.push_str(&format!("@Field{} int field{};\n", i, i));
        }
        
        source.push_str("}");
        
        let result = harness.parse("AnnotationBomb.java", &source);
        assert!(result.is_ok());
        
        Ok(())
    }
    
    #[test]
    fn test_lambda_and_anonymous_class_chaos() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class LambdaChaos {
    void chaos() {
        Runnable r = () -> {
            Runnable r2 = () -> {
                Runnable r3 = new Runnable() {
                    public void run() {
                        Runnable r4 = () -> {
                            class Local {
                                void method() {
                                    Runnable r5 = new Runnable() {
                                        public void run() {
                                            Runnable r6 = () -> System.out.println(
                                                new Object() {
                                                    public String toString() {
                                                        return new Supplier<String>() {
                                                            public String get() {
                                                                return ((Function<String, String>) (s -> s))
                                                                    .apply("chaos");
                                                            }
                                                        }.get();
                                                    }
                                                }
                                            );
                                        }
                                    };
                                }
                            }
                        };
                    }
                };
            };
        };
    }
}
"#;
        
        let result = harness.parse("LambdaChaos.java", source);
        assert!(result.is_ok());
        
        Ok(())
    }
    
    #[test]
    fn test_mixed_syntax_styles() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        
        // Mix old and new Java syntax styles
        let source = r#"
public class MixedStyles {
    // Old style array declaration
    int arr[] = new int[10];
    String strs[] = {"a", "b"};
    int[][] matrix = new int[10][];
    
    // Diamond operator variations
    List<String> list1 = new ArrayList<String>();
    List<String> list2 = new ArrayList<>();
    var list3 = new ArrayList<String>();
    
    // Switch variations
    int traditional(int x) {
        switch(x) {
            case 1: return 1;
            case 2: return 2;
            default: return 0;
        }
    }
    
    int expression(int x) {
        return switch(x) {
            case 1 -> 1;
            case 2 -> { yield 2; }
            default -> 0;
        };
    }
    
    // Try variations
    void tryVariations() {
        // Traditional try-catch
        try {
            risky();
        } catch (Exception e) {
            handle(e);
        } finally {
            cleanup();
        }
        
        // Try-with-resources
        try (var r = getResource()) {
            use(r);
        }
        
        // Multi-catch
        try {
            risky();
        } catch (IOException | SQLException e) {
            handle(e);
        }
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("MixedStyles.java", source)?;
        assert!(symbols.iter().any(|s| s.name == "MixedStyles"));
        
        Ok(())
    }
    
    #[test]
    fn test_type_inference_edge_cases() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class TypeInference {
    // var with complex types
    void testVar() {
        var x = 42;
        var y = "string";
        var z = new ArrayList<Map<String, List<Double>>>();
        var lambda = (Function<String, Integer>) s -> s.length();
        var array = new int[]{1, 2, 3};
        var anon = new Object() {
            int field = 42;
            void method() {}
        };
    }
    
    // Diamond operator edge cases
    void testDiamond() {
        Map<String, List<String>> map1 = new HashMap<>();
        var map2 = new HashMap<String, List<String>>();
        
        // Anonymous class with diamond
        List<String> list = new ArrayList<>() {
            @Override
            public boolean add(String s) {
                return super.add(s.toUpperCase());
            }
        };
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("TypeInference.java", source)?;
        assert!(symbols.iter().any(|s| s.name == "testVar"));
        assert!(symbols.iter().any(|s| s.name == "testDiamond"));
        
        Ok(())
    }
    
    #[test]
    fn test_recursive_data_structures() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
class Node<T> {
    T value;
    Node<T> next;
    Node<Node<T>> nested;
    List<Node<T>> children;
    Map<String, Node<T>> namedChildren;
    
    static class Tree<T> {
        T value;
        Tree<T> left;
        Tree<T> right;
        Tree<Tree<T>> metaTree;
    }
    
    interface Graph<T> {
        Set<Graph<T>> getNeighbors();
        Map<String, Graph<Graph<T>>> getMetaGraph();
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("Recursive.java", source)?;
        assert!(symbols.iter().any(|s| s.name == "Node"));
        assert!(symbols.iter().any(|s| s.name == "Tree"));
        assert!(symbols.iter().any(|s| s.name == "Graph"));
        
        Ok(())
    }
    
    #[test]
    fn test_weird_but_valid_syntax() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class WeirdButValid {
    // Unicode escapes in identifiers
    int \u0069\u006e\u0074\u0046\u0069\u0065\u006c\u0064 = 42;
    
    // Empty statements
    ;;;;;
    
    // Labeled blocks
    label1: {
        label2: {
            label3: break label1;
        }
    }
    
    // Weird but valid formatting
    public
    static
    final
    synchronized
    <T extends Comparable<? super T>>
    T
    weirdMethod
    (
        T
        param1
        ,
        T
        param2
    )
    throws
    Exception
    ,
    RuntimeException
    {
        return
        param1
        ;
    }
    
    // Constructor with class name as unicode escapes
    \u0057\u0065\u0069\u0072\u0064\u0042\u0075\u0074\u0056\u0061\u006c\u0069\u0064() {}
    
    // Hexadecimal and binary literals
    int hex = 0xDEAD_BEEF;
    int binary = 0b1010_1010_1010_1010;
    double sci = 1.23e-45;
    float hexFloat = 0x1.fffffeP+127f;
}
"#;
        
        let result = harness.parse("WeirdButValid.java", source);
        assert!(result.is_ok());
        
        Ok(())
    }
}