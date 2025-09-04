#[cfg(test)]
mod complex_tests {
    use crate::*;
    use anyhow::Result;
    
    #[test]
    fn test_complex_generics_with_bounds() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class ComplexGenerics<T extends Comparable<? super T> & Serializable> {
    private T value;
    
    public <U extends Number & Comparable<U>> U process(U input) {
        return input;
    }
    
    public void wildcardMethod(List<? extends Number> nums) {}
    
    public void lowerBound(List<? super Integer> nums) {}
    
    public <K extends Comparable<K>, V> Map<K, V> createMap() {
        return new HashMap<>();
    }
    
    public static <T> T covariant(Supplier<? extends T> supplier) {
        return supplier.get();
    }
    
    public static <T> void contravariant(Consumer<? super T> consumer, T item) {
        consumer.accept(item);
    }
}

interface Multibound<T extends Runnable & Cloneable & Comparable<T>> {
    T get();
}
"#;
        
        let (symbols, _, _) = harness.parse("test.java", source)?;
        
        let complex = symbols.iter().find(|s| s.name == "ComplexGenerics");
        assert!(complex.is_some());
        
        let methods = symbols.iter()
            .filter(|s| s.kind == protocol::SymbolKind::Method)
            .count();
        
        assert!(methods >= 6, "Should find multiple generic methods");
        
        Ok(())
    }
    
    #[test]
    fn test_nested_and_local_classes() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class OuterClass {
    private int outerField = 10;
    
    public class InnerClass {
        void accessOuter() {
            System.out.println(outerField);
        }
        
        class DoublyNested {
            void deepMethod() {}
        }
    }
    
    static class StaticNested {
        static class DeeplyStaticNested {
            void method() {}
        }
    }
    
    void methodWithLocalClass() {
        final int localVar = 20;
        
        class LocalClass {
            void useLocal() {
                System.out.println(localVar + outerField);
            }
            
            class LocalInnerClass {
                void veryLocal() {}
            }
        }
        
        LocalClass lc = new LocalClass();
        lc.useLocal();
    }
    
    Runnable anonymousExample = new Runnable() {
        class AnonymousInner {
            void method() {}
        }
        
        @Override
        public void run() {
            new AnonymousInner().method();
        }
    };
}
"#;
        
        let (symbols, _, _) = harness.parse("test.java", source)?;
        
        let outer = symbols.iter().find(|s| s.name == "OuterClass");
        assert!(outer.is_some());
        
        let classes = symbols.iter()
            .filter(|s| s.kind == protocol::SymbolKind::Class)
            .count();
        
        assert!(classes >= 1, "Should find at least the outer class");
        
        Ok(())
    }
    
    #[test]
    fn test_complex_annotations() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
import java.lang.annotation.*;

@Target({ElementType.TYPE, ElementType.METHOD})
@Retention(RetentionPolicy.RUNTIME)
@interface ComplexAnnotation {
    String value() default "";
    int[] numbers() default {1, 2, 3};
    Class<?> type() default Object.class;
    Priority priority() default Priority.MEDIUM;
    
    enum Priority {
        LOW, MEDIUM, HIGH
    }
}

@ComplexAnnotation(
    value = "test",
    numbers = {10, 20, 30},
    type = String.class,
    priority = ComplexAnnotation.Priority.HIGH
)
@Deprecated
@SuppressWarnings({"unchecked", "rawtypes"})
public class AnnotatedClass {
    
    @ComplexAnnotation
    @Override
    @SafeVarargs
    public final <T> void annotatedMethod(@NonNull T... args) {}
    
    @FunctionalInterface
    interface Processor {
        void process();
    }
}

@Repeatable(Container.class)
@interface Repeat {
    String value();
}

@interface Container {
    Repeat[] value();
}

@Repeat("first")
@Repeat("second")
class MultiAnnotated {}
"#;
        
        let (symbols, _, _) = harness.parse("test.java", source)?;
        
        let annotated = symbols.iter().find(|s| s.name == "AnnotatedClass");
        assert!(annotated.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_complex_enums() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public enum ComplexEnum implements Comparable<ComplexEnum> {
    FIRST(1, "First") {
        @Override
        public void abstractMethod() {
            System.out.println("First implementation");
        }
    },
    SECOND(2, "Second") {
        @Override
        public void abstractMethod() {
            System.out.println("Second implementation");
        }
    },
    THIRD(3, "Third") {
        @Override
        public void abstractMethod() {
            System.out.println("Third implementation");
        }
    };
    
    private final int code;
    private final String description;
    
    ComplexEnum(int code, String description) {
        this.code = code;
        this.description = description;
    }
    
    public abstract void abstractMethod();
    
    public int getCode() {
        return code;
    }
    
    public static ComplexEnum fromCode(int code) {
        for (ComplexEnum e : values()) {
            if (e.code == code) {
                return e;
            }
        }
        throw new IllegalArgumentException();
    }
}

enum SingletonEnum {
    INSTANCE;
    
    private final Object resource = new Object();
    
    public void doSomething() {
        synchronized(resource) {
            // Thread-safe singleton
        }
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("test.java", source)?;
        
        let complex_enum = symbols.iter().find(|s| s.name == "ComplexEnum");
        assert!(complex_enum.is_some());
        
        let methods = symbols.iter()
            .filter(|s| s.kind == protocol::SymbolKind::Method)
            .count();
        
        assert!(methods >= 2, "Should find enum methods");
        
        Ok(())
    }
    
    #[test]
    fn test_try_with_resources_complex() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
import java.io.*;
import java.sql.*;

public class ResourceManagement {
    public void multipleResources() throws Exception {
        try (FileInputStream fis = new FileInputStream("input.txt");
             InputStreamReader isr = new InputStreamReader(fis);
             BufferedReader br = new BufferedReader(isr);
             FileOutputStream fos = new FileOutputStream("output.txt");
             PrintWriter pw = new PrintWriter(fos)) {
            
            String line;
            while ((line = br.readLine()) != null) {
                pw.println(line.toUpperCase());
            }
        } catch (IOException | NullPointerException e) {
            e.printStackTrace();
        } finally {
            System.out.println("Cleanup complete");
        }
    }
    
    public void nestedTryWithResources() throws SQLException {
        try (Connection conn = getConnection()) {
            try (Statement stmt = conn.createStatement();
                 ResultSet rs = stmt.executeQuery("SELECT * FROM users")) {
                while (rs.next()) {
                    process(rs);
                }
            }
        }
    }
    
    private Connection getConnection() throws SQLException {
        return null;
    }
    
    private void process(ResultSet rs) throws SQLException {}
}

class CustomResource implements AutoCloseable {
    @Override
    public void close() throws Exception {
        System.out.println("Closing custom resource");
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("test.java", source)?;
        
        let resource_mgmt = symbols.iter().find(|s| s.name == "ResourceManagement");
        assert!(resource_mgmt.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_complex_switch_patterns() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class PatternMatching {
    public String processObject(Object obj) {
        return switch (obj) {
            case Integer i when i > 0 -> "Positive: " + i;
            case Integer i when i < 0 -> "Negative: " + i;
            case Integer i -> "Zero";
            case String s when s.length() > 10 -> "Long string";
            case String s -> "Short string: " + s;
            case Double d -> "Double: " + d;
            case null -> "Null value";
            default -> "Unknown type";
        };
    }
    
    sealed interface Shape permits Circle, Rectangle, Triangle {}
    
    record Circle(double radius) implements Shape {}
    record Rectangle(double width, double height) implements Shape {}
    record Triangle(double base, double height) implements Shape {}
    
    public double calculateArea(Shape shape) {
        return switch (shape) {
            case Circle(var r) -> Math.PI * r * r;
            case Rectangle(var w, var h) -> w * h;
            case Triangle(var b, var h) -> 0.5 * b * h;
        };
    }
    
    public void enhancedSwitch(int day) {
        var result = switch (day) {
            case 1, 2, 3, 4, 5 -> {
                System.out.println("Weekday");
                yield "Work";
            }
            case 6, 7 -> {
                System.out.println("Weekend");
                yield "Rest";
            }
            default -> throw new IllegalArgumentException();
        };
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("test.java", source)?;
        
        let pattern = symbols.iter().find(|s| s.name == "PatternMatching");
        assert!(pattern.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_modules_and_packages() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
module com.example.app {
    requires java.base;
    requires java.sql;
    requires transitive java.logging;
    
    exports com.example.api;
    exports com.example.impl to com.example.test;
    
    opens com.example.model;
    opens com.example.internal to com.fasterxml.jackson.databind;
    
    uses com.example.spi.Service;
    provides com.example.spi.Service with com.example.impl.ServiceImpl;
}

package com.example.api;

public interface PublicAPI {
    void publicMethod();
}

package com.example.impl;

class InternalImpl implements PublicAPI {
    @Override
    public void publicMethod() {
        // Implementation
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("test.java", source)?;
        
        // Module declarations might not be fully supported
        let api = symbols.iter()
            .filter(|s| s.kind == protocol::SymbolKind::Class || 
                        s.kind == protocol::SymbolKind::Interface)
            .count();
        
        assert!(api >= 1);
        
        Ok(())
    }
    
    #[test]
    fn test_varargs_and_arrays() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class VarargsTest {
    @SafeVarargs
    public final <T> void safeVarargs(T... args) {
        for (T arg : args) {
            process(arg);
        }
    }
    
    public void mixedParams(String first, int second, Object... rest) {
        System.out.printf("%s %d", first, second);
        for (Object obj : rest) {
            System.out.println(obj);
        }
    }
    
    public void arrayParams(int[] numbers, String[][] matrix, Object[][][] cube) {
        int[][][] multiDim = new int[10][20][30];
        String[] strs = {"a", "b", "c"};
        Object[][] jagged = {
            {1, 2, 3},
            {"a", "b"},
            {true}
        };
    }
    
    private <T> void process(T item) {}
}
"#;
        
        let (symbols, _, _) = harness.parse("test.java", source)?;
        
        let varargs = symbols.iter().find(|s| s.name == "VarargsTest");
        assert!(varargs.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_static_imports_and_blocks() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
import static java.lang.Math.*;
import static java.util.Collections.emptyList;
import static java.lang.System.out;

public class StaticFeatures {
    static {
        System.out.println("Static initializer 1");
    }
    
    private static final int CONSTANT = calculateConstant();
    
    static {
        System.out.println("Static initializer 2");
    }
    
    private static int calculateConstant() {
        return (int) (PI * E);
    }
    
    {
        // Instance initializer
        out.println("Instance created");
    }
    
    public void useStaticImports() {
        double result = sin(PI / 2) + cos(0) + sqrt(16);
        var list = emptyList();
    }
    
    public static class StaticInner {
        static {
            out.println("StaticInner initialized");
        }
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("test.java", source)?;
        
        let static_features = symbols.iter().find(|s| s.name == "StaticFeatures");
        assert!(static_features.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_instanceof_patterns() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class InstanceofPatterns {
    public void oldStyle(Object obj) {
        if (obj instanceof String) {
            String s = (String) obj;
            System.out.println(s.length());
        }
    }
    
    public void patternVariable(Object obj) {
        if (obj instanceof String s) {
            System.out.println(s.length());
        } else if (obj instanceof Integer i && i > 0) {
            System.out.println("Positive: " + i);
        } else if (obj instanceof List<?> list && !list.isEmpty()) {
            System.out.println("Non-empty list");
        }
    }
    
    public void negatedPattern(Object obj) {
        if (!(obj instanceof String s)) {
            System.out.println("Not a string");
            return;
        }
        // s is in scope here
        System.out.println(s.toUpperCase());
    }
    
    public boolean complexCondition(Object obj) {
        return obj instanceof String s && s.length() > 10 ||
               obj instanceof Integer i && i > 100 ||
               obj instanceof List<?> l && l.size() > 5;
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("test.java", source)?;
        
        let patterns = symbols.iter().find(|s| s.name == "InstanceofPatterns");
        assert!(patterns.is_some());
        
        Ok(())
    }
}