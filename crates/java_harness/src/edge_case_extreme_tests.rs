#[cfg(test)]
mod edge_case_extreme_tests {
    use crate::*;
    use anyhow::Result;
    
    #[test]
    fn test_deeply_nested_inner_classes() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        
        let mut source = String::from("public class Outer {\n");
        let mut indent = "    ";
        
        // Create 20 levels of nested inner classes
        for i in 0..20 {
            source.push_str(&format!("{}class Inner{} {{\n", indent, i));
            indent = format!("    {}", indent).leak();
        }
        
        // Close all classes
        for _ in 0..20 {
            indent = &indent[4..];
            source.push_str(&format!("{}}}\n", indent));
        }
        source.push_str("}\n");
        
        let result = harness.parse("Nested.java", &source);
        assert!(result.is_ok(), "Should handle deeply nested inner classes");
        
        Ok(())
    }
    
    #[test]
    fn test_unicode_identifiers_everywhere() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class 日本語クラス {
    private int αβγ = 42;
    private String Σομε_Τεχτ = "hello";
    
    public void методРаботы() {
        int العربية = 10;
        double π = 3.14159;
        String emoji😀 = "works"; // Some parsers might fail here
    }
    
    class 内部クラス {
        void 中文方法() {
            System.out.println("Unicode everywhere!");
        }
    }
    
    enum Κατάσταση {
        ΕΝΕΡΓΌ,
        ΑΝΕΝΕΡΓΌ
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("Unicode.java", source)?;
        
        assert!(symbols.len() > 0, "Should parse unicode identifiers");
        
        Ok(())
    }
    
    #[test]
    fn test_anonymous_class_madness() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class AnonymousMadness {
    void test() {
        Runnable r1 = new Runnable() {
            public void run() {
                Runnable r2 = new Runnable() {
                    public void run() {
                        Object obj = new Object() {
                            @Override
                            public String toString() {
                                return new StringBuilder() {
                                    {
                                        // Instance initializer in anonymous class
                                        append("Hello");
                                        append(" ");
                                        append("World");
                                    }
                                }.toString();
                            }
                        }.toString();
                    }
                };
            }
        };
        
        // Anonymous class with generic type
        List<String> list = new ArrayList<String>() {{
            add("item1");
            add("item2");
        }};
        
        // Anonymous interface implementation
        Comparator<Integer> comp = new Comparator<Integer>() {
            @Override
            public int compare(Integer a, Integer b) {
                return new Comparator<Integer>() {
                    public int compare(Integer x, Integer y) {
                        return x - y;
                    }
                }.compare(a, b);
            }
        };
    }
}
"#;
        
        let result = harness.parse("Anonymous.java", source);
        assert!(result.is_ok(), "Should handle complex anonymous classes");
        
        Ok(())
    }
    
    #[test]
    fn test_type_parameter_bounds_explosion() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class TypeExplosion<
    T extends Comparable<? super T> & Serializable & Cloneable,
    U extends Map<String, ? extends List<? super Set<? extends T>>>,
    V extends Function<
        ? super Map<String, ? extends List<? super T>>,
        ? extends Map<? super String, ? extends Set<? super U>>
    >,
    W extends T & U,  // Multiple bounds
    X extends Enum<X> & Comparable<X>
> {
    // Wildcard captures
    void process(
        List<? extends T> list1,
        List<? super T> list2,
        Map<? extends String, ? super List<? extends T>> map
    ) {
        // Recursive type parameter
        class Local<Y extends Local<Y>> {}
        
        // Self-referential generics
        class Node<N extends Node<N>> {
            N next;
        }
    }
    
    // Method with complex type parameters
    public <
        A extends T,
        B extends U & Comparable<? super B>,
        C extends Map<A, B> & Serializable
    > C complexMethod(
        Function<? super A, ? extends B> fn,
        Supplier<? extends C> supplier
    ) {
        return null;
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("TypeExplosion.java", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "TypeExplosion"));
        assert!(symbols.iter().any(|s| s.name == "complexMethod"));
        
        Ok(())
    }
    
    #[test]
    fn test_annotation_processor_torture() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
@Target({ElementType.TYPE, ElementType.METHOD, ElementType.FIELD})
@Retention(RetentionPolicy.RUNTIME)
@Repeatable(Container.class)
@interface Complex {
    String value() default "";
    int[] numbers() default {1, 2, 3};
    Class<?>[] types() default {String.class, Integer.class};
    Thread.State state() default Thread.State.NEW;
    @interface Nested {
        String data();
    }
    Nested nested() default @Nested(data = "default");
}

@Complex("one")
@Complex("two")
@Complex("three")
@SuppressWarnings({"unchecked", "rawtypes", "deprecation"})
@Deprecated(since = "1.0", forRemoval = true)
public class AnnotationTorture {
    
    @Complex(
        value = "field",
        numbers = {1, 2, 3, 4, 5},
        types = {String.class, Integer.class, Double.class},
        state = Thread.State.RUNNABLE,
        nested = @Complex.Nested(data = "nested")
    )
    private String field;
    
    @SafeVarargs
    @SuppressWarnings("varargs")
    public final <T> void varargs(T... args) {}
    
    // Type annotations (Java 8+)
    private @NonNull String nonNull;
    private List<@NonNull String> list;
    private String @NonNull [] array;
    
    void method() throws @NonNull Exception {
        @NonNull String local = "test";
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("Annotations.java", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "AnnotationTorture"));
        
        Ok(())
    }
    
    #[test]
    fn test_switch_expression_patterns() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class SwitchPatterns {
    // Traditional switch
    String traditional(int day) {
        switch (day) {
            case 1:
            case 2:
            case 3:
            case 4:
            case 5:
                return "Weekday";
            case 6:
            case 7:
                return "Weekend";
            default:
                return "Invalid";
        }
    }
    
    // Switch expression (Java 14+)
    String expression(int day) {
        return switch (day) {
            case 1, 2, 3, 4, 5 -> "Weekday";
            case 6, 7 -> "Weekend";
            default -> {
                System.out.println("Invalid day");
                yield "Invalid";
            }
        };
    }
    
    // Pattern matching (Java 17+)
    String pattern(Object obj) {
        return switch (obj) {
            case Integer i when i > 0 -> "Positive: " + i;
            case Integer i -> "Non-positive: " + i;
            case String s when s.length() > 5 -> "Long string: " + s;
            case String s -> "Short string: " + s;
            case null -> "Null value";
            case int[] arr -> "Array length: " + arr.length;
            default -> "Unknown";
        };
    }
    
    // Record patterns (Java 19+)
    record Point(int x, int y) {}
    record Line(Point start, Point end) {}
    
    String recordPattern(Object obj) {
        return switch (obj) {
            case Point(int x, int y) -> "Point: " + x + ", " + y;
            case Line(Point(var x1, var y1), Point(var x2, var y2)) -> 
                "Line from (" + x1 + "," + y1 + ") to (" + x2 + "," + y2 + ")";
            default -> "Unknown";
        };
    }
}
"#;
        
        let result = harness.parse("Switch.java", source);
        assert!(result.is_ok(), "Should handle various switch patterns");
        
        Ok(())
    }
    
    #[test]
    fn test_sealed_class_hierarchies() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
// Sealed classes (Java 17+)
public sealed class Shape 
    permits Circle, Rectangle, Triangle {
    abstract double area();
}

final class Circle extends Shape {
    private final double radius;
    
    Circle(double radius) {
        this.radius = radius;
    }
    
    @Override
    double area() {
        return Math.PI * radius * radius;
    }
}

non-sealed class Rectangle extends Shape {
    private final double width, height;
    
    Rectangle(double width, double height) {
        this.width = width;
        this.height = height;
    }
    
    @Override
    double area() {
        return width * height;
    }
}

sealed class Triangle extends Shape 
    permits EquilateralTriangle, IsoscelesTriangle {
    @Override
    double area() {
        return 0;
    }
}

final class EquilateralTriangle extends Triangle {}
final class IsoscelesTriangle extends Triangle {}

// Sealed interface
sealed interface Vehicle permits Car, Truck, Motorcycle {}

record Car(String brand) implements Vehicle {}
record Truck(int capacity) implements Vehicle {}
record Motorcycle(boolean hasSidecar) implements Vehicle {}
"#;
        
        let (symbols, edges, _) = harness.parse("Sealed.java", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "Shape"));
        assert!(symbols.iter().any(|s| s.name == "Circle"));
        assert!(edges.iter().any(|e| e.edge_type == EdgeType::Extends));
        
        Ok(())
    }
    
    #[test]
    fn test_record_edge_cases() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
// Records (Java 14+)
public record SimpleRecord(String name, int age) {}

// Record with custom constructor
record ValidatedRecord(String name, int age) {
    ValidatedRecord {
        if (age < 0) throw new IllegalArgumentException("Age cannot be negative");
        // Compact constructor
    }
}

// Record with methods
record ComplexRecord(String name, List<String> items) {
    // Custom constructor
    ComplexRecord(String name) {
        this(name, new ArrayList<>());
    }
    
    // Methods
    int itemCount() {
        return items.size();
    }
    
    // Static fields and methods
    static final ComplexRecord EMPTY = new ComplexRecord("empty", List.of());
    
    static ComplexRecord empty() {
        return EMPTY;
    }
    
    // Nested types
    enum Status { ACTIVE, INACTIVE }
    
    record Nested(Status status) {}
}

// Generic record
record Pair<T, U>(T first, U second) implements Comparable<Pair<T, U>> {
    @Override
    public int compareTo(Pair<T, U> other) {
        return 0;
    }
}

// Record with annotations
record AnnotatedRecord(
    @NotNull String name,
    @Min(0) @Max(150) int age,
    @Email String email
) {}
"#;
        
        let (symbols, _, _) = harness.parse("Records.java", source)?;
        
        assert!(symbols.iter().any(|s| s.name == "SimpleRecord"));
        assert!(symbols.iter().any(|s| s.name == "ComplexRecord"));
        
        Ok(())
    }
    
    #[test]
    fn test_text_blocks_edge_cases() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r####"
public class TextBlocks {
    // Basic text block
    String html = """
        <html>
            <body>
                <p>Hello, World!</p>
            </body>
        </html>
        """;
    
    // With escapes
    String json = """
        {
            "name": "John",
            "age": 30,
            "quote": "He said, \\"Hello!\\""
        }
        """;
    
    // Empty text block
    String empty = """
        """;
    
    // Single line
    String single = """
        Single line text block""";
    
    // With line continuations
    String continued = """
        This is a very \
        long line that \
        continues""";
    
    // Nested quotes
    String nested = """
        String text = \"""
            Nested text block
            \""";
        """;
    
    // SQL query
    String sql = """
        SELECT * FROM users
        WHERE age > 18
            AND status = 'active'
        ORDER BY created_at DESC
        """;
    
    // With trailing spaces (should be stripped)
    String trailing = """
        Line with spaces    
        Another line     
        """;
}
"####;
        
        let result = harness.parse("TextBlocks.java", source);
        assert!(result.is_ok(), "Should handle text blocks");
        
        Ok(())
    }
    
    #[test]
    fn test_var_keyword_inference() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class VarInference {
    void test() {
        // Basic var
        var i = 42;
        var s = "string";
        var list = new ArrayList<String>();
        
        // var with diamond operator
        var map = new HashMap<>();
        
        // var with anonymous class
        var anon = new Object() {
            int x = 10;
            String method() { return "test"; }
        };
        
        // var in loops
        var numbers = List.of(1, 2, 3, 4, 5);
        for (var num : numbers) {
            System.out.println(num);
        }
        
        for (var j = 0; j < 10; j++) {
            System.out.println(j);
        }
        
        // var with array
        var array = new int[]{1, 2, 3};
        
        // var with lambda (not allowed but should not crash)
        // var lambda = (String x) -> x.length();  // Compilation error
        
        // var with method reference (not allowed)
        // var ref = String::length;  // Compilation error
        
        // var with null (not allowed)
        // var n = null;  // Compilation error
        
        // var with ternary
        var result = true ? "yes" : "no";
        
        // Try-with-resources
        try (var reader = new BufferedReader(new FileReader("file.txt"))) {
            var line = reader.readLine();
        }
    }
}
"#;
        
        let result = harness.parse("Var.java", source);
        assert!(result.is_ok(), "Should handle var keyword");
        
        Ok(())
    }
    
    #[test]
    fn test_module_system() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
// module-info.java
module com.example.myapp {
    // Requires
    requires java.base;
    requires transitive java.logging;
    requires static java.sql;
    
    // Exports
    exports com.example.myapp.api;
    exports com.example.myapp.internal to com.example.trusted;
    
    // Opens for reflection
    opens com.example.myapp.model;
    opens com.example.myapp.util to java.base;
    
    // Uses
    uses com.example.myapp.spi.Plugin;
    
    // Provides
    provides com.example.myapp.spi.Plugin 
        with com.example.myapp.impl.DefaultPlugin;
    
    provides java.sql.Driver
        with com.example.myapp.impl.CustomDriver,
             com.example.myapp.impl.AnotherDriver;
}
"#;
        
        let result = harness.parse("module-info.java", source);
        assert!(result.is_ok(), "Should handle module declarations");
        
        Ok(())
    }
    
    #[test]
    fn test_try_with_resources_variations() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class TryWithResources {
    void test() throws Exception {
        // Single resource
        try (FileReader reader = new FileReader("file.txt")) {
            // use reader
        }
        
        // Multiple resources
        try (FileReader reader = new FileReader("input.txt");
             FileWriter writer = new FileWriter("output.txt");
             Scanner scanner = new Scanner(System.in)) {
            // use resources
        }
        
        // With catch and finally
        try (BufferedReader br = new BufferedReader(new FileReader("file.txt"))) {
            br.readLine();
        } catch (IOException e) {
            e.printStackTrace();
        } finally {
            System.out.println("Done");
        }
        
        // Effectively final variables (Java 9+)
        FileReader reader1 = new FileReader("file1.txt");
        FileReader reader2 = new FileReader("file2.txt");
        
        try (reader1; reader2) {
            // use readers
        }
        
        // With var (Java 10+)
        try (var input = new FileInputStream("input.dat");
             var output = new FileOutputStream("output.dat")) {
            input.transferTo(output);
        }
        
        // Custom AutoCloseable
        class CustomResource implements AutoCloseable {
            @Override
            public void close() {
                System.out.println("Closing");
            }
        }
        
        try (CustomResource resource = new CustomResource()) {
            // use resource
        }
    }
}
"#;
        
        let result = harness.parse("TryWithResources.java", source);
        assert!(result.is_ok(), "Should handle try-with-resources variations");
        
        Ok(())
    }
    
    #[test]
    fn test_method_references_and_constructors() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class MethodReferences {
    void test() {
        // Static method reference
        Function<String, Integer> parser = Integer::parseInt;
        
        // Instance method reference on type
        Function<String, String> upper = String::toUpperCase;
        
        // Instance method reference on object
        String str = "test";
        Supplier<String> supplier = str::toUpperCase;
        
        // Constructor reference
        Supplier<List<String>> listFactory = ArrayList::new;
        Function<Integer, List<String>> sizedListFactory = ArrayList::new;
        
        // Array constructor reference
        IntFunction<int[]> arrayFactory = int[]::new;
        
        // Generic constructor reference
        Function<String, Optional<String>> optionalFactory = Optional::<String>new;
        
        // Method reference to this
        Runnable r = this::instanceMethod;
        
        // Method reference to super
        Runnable r2 = super::toString;
        
        // Complex chains
        list.stream()
            .map(String::trim)
            .filter(StringUtils::isNotEmpty)
            .map(Integer::valueOf)
            .collect(Collectors.toList());
    }
    
    void instanceMethod() {}
}
"#;
        
        let result = harness.parse("MethodReferences.java", source);
        assert!(result.is_ok(), "Should handle method references");
        
        Ok(())
    }
    
    #[test]
    fn test_intersection_types() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class IntersectionTypes {
    // Intersection type in generics
    class Multi<T extends Serializable & Comparable<T> & Cloneable> {
        T value;
    }
    
    // Intersection in method bounds
    public <T extends Number & Comparable<T>> T min(T a, T b) {
        return a.compareTo(b) < 0 ? a : b;
    }
    
    // Cast to intersection type
    void test(Object obj) {
        // Cast to intersection type (Java 8+)
        Runnable r = (Runnable & Serializable) () -> System.out.println("Hello");
        
        // Multiple bounds in lambda
        process((Runnable & Serializable) () -> {});
    }
    
    void process(Object obj) {}
    
    // Intersection in catch (multi-catch)
    void multiCatch() {
        try {
            riskyOperation();
        } catch (IOException | SQLException e) {
            // e is effectively final and has intersection type
            e.printStackTrace();
        }
    }
    
    void riskyOperation() throws IOException, SQLException {}
}
"#;
        
        let result = harness.parse("Intersection.java", source);
        assert!(result.is_ok(), "Should handle intersection types");
        
        Ok(())
    }
}