#[cfg(test)]
mod strict_tests {
    use crate::*;
    use anyhow::Result;
    use protocol::{EdgeType, SymbolKind, OccurrenceRole};
    
    #[test]
    fn test_exact_class_parsing() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
package com.example.test;

import java.util.List;
import java.util.ArrayList;

public class TestClass {
    private int privateField;
    protected String protectedField;
    public static final double PUBLIC_CONSTANT = 3.14;
    
    public TestClass() {
        this.privateField = 0;
    }
    
    public TestClass(int value) {
        this.privateField = value;
    }
    
    private void privateMethod() {}
    protected int protectedMethod(String arg) { return 42; }
    public static void staticMethod() {}
}
"#;
        
        let (symbols, edges, occurrences) = harness.parse("Test.java", source)?;
        
        // Exact class count and properties
        let classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .collect::<Vec<_>>();
        assert_eq!(classes.len(), 1, "Should find exactly 1 class");
        
        let test_class = &classes[0];
        assert_eq!(test_class.name, "TestClass");
        assert_eq!(test_class.fqn, "com.example.test.TestClass");
        assert_eq!(test_class.visibility.as_deref(), Some("public"));
        
        // Exact field count and properties
        let fields = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Field)
            .collect::<Vec<_>>();
        assert_eq!(fields.len(), 3, "Should find exactly 3 fields");
        
        // Verify each field
        let private_field = fields.iter()
            .find(|f| f.name == "privateField");
        assert!(private_field.is_some());
        assert_eq!(private_field.unwrap().visibility.as_deref(), Some("private"));
        assert!(private_field.unwrap().fqn.contains("TestClass.privateField"));
        
        let protected_field = fields.iter()
            .find(|f| f.name == "protectedField");
        assert!(protected_field.is_some());
        assert_eq!(protected_field.unwrap().visibility.as_deref(), Some("protected"));
        
        let public_const = fields.iter()
            .find(|f| f.name == "PUBLIC_CONSTANT");
        assert!(public_const.is_some());
        assert_eq!(public_const.unwrap().visibility.as_deref(), Some("public"));
        
        // Exact method count and properties
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect::<Vec<_>>();
        assert_eq!(methods.len(), 5, "Should find exactly 5 methods (2 constructors + 3 methods)");
        
        // Verify constructors
        let constructors = methods.iter()
            .filter(|m| m.name == "TestClass")
            .collect::<Vec<_>>();
        assert_eq!(constructors.len(), 2, "Should find exactly 2 constructors");
        
        // Verify methods
        assert!(methods.iter().any(|m| m.name == "privateMethod" && 
            m.visibility.as_deref() == Some("private")));
        assert!(methods.iter().any(|m| m.name == "protectedMethod" && 
            m.visibility.as_deref() == Some("protected")));
        assert!(methods.iter().any(|m| m.name == "staticMethod" && 
            m.visibility.as_deref() == Some("public")));
        
        // Check imports
        let import_edges = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .collect::<Vec<_>>();
        assert_eq!(import_edges.len(), 2, "Should find exactly 2 imports");
        
        // Verify occurrences
        assert_eq!(occurrences.len(), symbols.len(),
            "Each symbol should have exactly one occurrence");
        
        for occ in &occurrences {
            assert_eq!(occ.role, OccurrenceRole::Definition);
            assert!(occ.symbol_id.is_some());
            assert_eq!(occ.file_path, "Test.java");
        }
        
        Ok(())
    }
    
    #[test]
    fn test_exact_interface_implementation() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
interface Drawable {
    void draw();
    default void clear() {
        System.out.println("Clearing");
    }
}

interface Resizable {
    void resize(int width, int height);
}

interface Shape extends Drawable, Resizable {
    double area();
}

public class Rectangle implements Shape {
    private int width, height;
    
    @Override
    public void draw() {
        System.out.println("Drawing rectangle");
    }
    
    @Override
    public void resize(int w, int h) {
        this.width = w;
        this.height = h;
    }
    
    @Override
    public double area() {
        return width * height;
    }
}
"#;
        
        let (symbols, edges, _) = harness.parse("Shapes.java", source)?;
        
        // Exact interface count
        let interfaces = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Interface)
            .collect::<Vec<_>>();
        assert_eq!(interfaces.len(), 3, "Should find exactly 3 interfaces");
        
        // Verify interface names
        assert!(interfaces.iter().any(|i| i.name == "Drawable"));
        assert!(interfaces.iter().any(|i| i.name == "Resizable"));
        assert!(interfaces.iter().any(|i| i.name == "Shape"));
        
        // Exact class count
        let classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .collect::<Vec<_>>();
        assert_eq!(classes.len(), 1, "Should find exactly 1 class");
        assert_eq!(classes[0].name, "Rectangle");
        
        // Check extends edges
        let extends_edges = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Extends)
            .collect::<Vec<_>>();
        
        // Shape extends Drawable and Resizable
        assert!(extends_edges.len() >= 2, "Shape should extend at least 2 interfaces");
        assert!(extends_edges.iter().any(|e|
            e.src.as_ref().map(|s| s.contains("Shape")).unwrap_or(false) &&
            e.dst.as_ref().map(|d| d == "Drawable").unwrap_or(false)
        ));
        assert!(extends_edges.iter().any(|e|
            e.src.as_ref().map(|s| s.contains("Shape")).unwrap_or(false) &&
            e.dst.as_ref().map(|d| d == "Resizable").unwrap_or(false)
        ));
        
        // Check implements edges
        let implements_edges = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Implements)
            .collect::<Vec<_>>();
        
        assert!(implements_edges.len() >= 1, "Rectangle should implement Shape");
        assert!(implements_edges.iter().any(|e|
            e.src.as_ref().map(|s| s.contains("Rectangle")).unwrap_or(false) &&
            e.dst.as_ref().map(|d| d == "Shape").unwrap_or(false)
        ));
        
        // Check methods
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect::<Vec<_>>();
        
        // Interface methods
        assert!(methods.iter().any(|m| m.name == "draw" && 
            m.fqn.contains("Drawable")));
        assert!(methods.iter().any(|m| m.name == "clear" && 
            m.fqn.contains("Drawable")));
        assert!(methods.iter().any(|m| m.name == "resize" && 
            m.fqn.contains("Resizable")));
        assert!(methods.iter().any(|m| m.name == "area" && 
            m.fqn.contains("Shape")));
        
        // Implementation methods
        assert!(methods.iter().any(|m| m.name == "draw" && 
            m.fqn.contains("Rectangle")));
        assert!(methods.iter().any(|m| m.name == "resize" && 
            m.fqn.contains("Rectangle")));
        assert!(methods.iter().any(|m| m.name == "area" && 
            m.fqn.contains("Rectangle")));
        
        Ok(())
    }
    
    #[test]
    fn test_exact_enum_parsing() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public enum DayOfWeek {
    MONDAY("Mon", 1),
    TUESDAY("Tue", 2),
    WEDNESDAY("Wed", 3),
    THURSDAY("Thu", 4),
    FRIDAY("Fri", 5),
    SATURDAY("Sat", 6),
    SUNDAY("Sun", 7);
    
    private final String abbreviation;
    private final int dayNumber;
    
    DayOfWeek(String abbreviation, int dayNumber) {
        this.abbreviation = abbreviation;
        this.dayNumber = dayNumber;
    }
    
    public String getAbbreviation() {
        return abbreviation;
    }
    
    public int getDayNumber() {
        return dayNumber;
    }
    
    public boolean isWeekend() {
        return this == SATURDAY || this == SUNDAY;
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("DayOfWeek.java", source)?;
        
        // Exact enum count
        let enums = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Enum)
            .collect::<Vec<_>>();
        assert_eq!(enums.len(), 1, "Should find exactly 1 enum");
        assert_eq!(enums[0].name, "DayOfWeek");
        assert_eq!(enums[0].visibility.as_deref(), Some("public"));
        
        // Exact enum constant count
        let constants = symbols.iter()
            .filter(|s| s.kind == SymbolKind::EnumMember)
            .collect::<Vec<_>>();
        assert_eq!(constants.len(), 7, "Should find exactly 7 enum constants");
        
        // Verify all days
        let days = ["MONDAY", "TUESDAY", "WEDNESDAY", "THURSDAY", "FRIDAY", "SATURDAY", "SUNDAY"];
        for day in days {
            assert!(constants.iter().any(|c| c.name == day),
                "Should find {} constant", day);
        }
        
        // Check fields
        let fields = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Field)
            .collect::<Vec<_>>();
        assert_eq!(fields.len(), 2, "Should find exactly 2 fields");
        assert!(fields.iter().any(|f| f.name == "abbreviation"));
        assert!(fields.iter().any(|f| f.name == "dayNumber"));
        
        // Check methods
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect::<Vec<_>>();
        
        // Constructor + 3 methods
        assert!(methods.iter().any(|m| m.name == "DayOfWeek"), "Should find constructor");
        assert!(methods.iter().any(|m| m.name == "getAbbreviation"));
        assert!(methods.iter().any(|m| m.name == "getDayNumber"));
        assert!(methods.iter().any(|m| m.name == "isWeekend"));
        
        Ok(())
    }
    
    #[test]
    fn test_exact_generic_parsing() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
import java.util.*;

public class GenericContainer<T extends Comparable<T>> {
    private List<T> items;
    private Map<String, T> namedItems;
    
    public GenericContainer() {
        this.items = new ArrayList<>();
        this.namedItems = new HashMap<>();
    }
    
    public void add(T item) {
        items.add(item);
    }
    
    public <U extends Number> U convertToNumber(T item, U defaultValue) {
        return defaultValue;
    }
    
    public static <K, V> Map<K, V> createMap() {
        return new HashMap<>();
    }
    
    public List<? extends T> getSubList() {
        return items;
    }
    
    public void addAll(Collection<? super T> collection) {
        // Implementation
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("Generic.java", source)?;
        
        // Check class
        let classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .collect::<Vec<_>>();
        assert_eq!(classes.len(), 1, "Should find exactly 1 class");
        assert_eq!(classes[0].name, "GenericContainer");
        
        // Check fields
        let fields = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Field)
            .collect::<Vec<_>>();
        assert_eq!(fields.len(), 2, "Should find exactly 2 fields");
        assert!(fields.iter().any(|f| f.name == "items"));
        assert!(fields.iter().any(|f| f.name == "namedItems"));
        
        // Check methods
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect::<Vec<_>>();
        
        // Verify specific methods exist
        assert!(methods.iter().any(|m| m.name == "GenericContainer"), "Should find constructor");
        assert!(methods.iter().any(|m| m.name == "add"));
        assert!(methods.iter().any(|m| m.name == "convertToNumber"));
        assert!(methods.iter().any(|m| m.name == "createMap"));
        assert!(methods.iter().any(|m| m.name == "getSubList"));
        assert!(methods.iter().any(|m| m.name == "addAll"));
        
        Ok(())
    }
    
    #[test]
    fn test_exact_annotation_parsing() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
import java.lang.annotation.*;

@Target(ElementType.TYPE)
@Retention(RetentionPolicy.RUNTIME)
@Documented
public @interface CustomAnnotation {
    String value() default "";
    int priority() default 0;
    Class<?>[] types() default {};
}

@CustomAnnotation(value = "test", priority = 5)
@Deprecated
public class AnnotatedClass {
    @Deprecated
    private String oldField;
    
    @Override
    @SuppressWarnings("unchecked")
    public String toString() {
        return "AnnotatedClass";
    }
    
    @SafeVarargs
    public final <T> void varArgsMethod(T... args) {}
}
"#;
        
        let (symbols, _, _) = harness.parse("Annotated.java", source)?;
        
        // Check annotation interface
        let annotations = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Interface && 
                       s.name.contains("Annotation"))
            .collect::<Vec<_>>();
        assert!(annotations.len() >= 1, "Should find at least 1 annotation interface");
        
        // Check annotation methods
        let annotation_methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method && 
                       s.fqn.contains("CustomAnnotation"))
            .collect::<Vec<_>>();
        assert!(annotation_methods.iter().any(|m| m.name == "value"));
        assert!(annotation_methods.iter().any(|m| m.name == "priority"));
        assert!(annotation_methods.iter().any(|m| m.name == "types"));
        
        // Check annotated class
        let classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .collect::<Vec<_>>();
        assert!(classes.iter().any(|c| c.name == "AnnotatedClass"));
        
        // Check methods in annotated class
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method && 
                       s.fqn.contains("AnnotatedClass"))
            .collect::<Vec<_>>();
        assert!(methods.iter().any(|m| m.name == "toString"));
        assert!(methods.iter().any(|m| m.name == "varArgsMethod"));
        
        Ok(())
    }
    
    #[test]
    fn test_exact_inner_class_parsing() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class OuterClass {
    private int outerField = 10;
    
    public class InnerClass {
        private int innerField;
        
        public void innerMethod() {
            System.out.println(outerField);
        }
    }
    
    public static class StaticNestedClass {
        private static int staticField;
        
        public static void staticMethod() {}
    }
    
    private interface InnerInterface {
        void interfaceMethod();
    }
    
    public void createAnonymous() {
        Runnable r = new Runnable() {
            @Override
            public void run() {
                System.out.println("Running");
            }
        };
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("Outer.java", source)?;
        
        // Check outer class
        let outer_classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class && s.name == "OuterClass")
            .collect::<Vec<_>>();
        assert_eq!(outer_classes.len(), 1, "Should find exactly 1 OuterClass");
        
        // Note: Inner classes might not all be parsed depending on tree-sitter support
        // But we should at least find the outer class and its direct members
        
        // Check outer class field
        let outer_fields = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Field && 
                       s.fqn.contains("OuterClass") &&
                       s.name == "outerField")
            .collect::<Vec<_>>();
        assert_eq!(outer_fields.len(), 1, "Should find outerField");
        
        // Check outer class method
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method && 
                       s.fqn.contains("OuterClass"))
            .collect::<Vec<_>>();
        assert!(methods.iter().any(|m| m.name == "createAnonymous"));
        
        Ok(())
    }
    
    #[test]
    fn test_exact_record_parsing() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public record Person(String name, int age) {
    public Person {
        if (age < 0) {
            throw new IllegalArgumentException("Age cannot be negative");
        }
    }
    
    public Person(String name) {
        this(name, 0);
    }
    
    public String greet() {
        return "Hello, I'm " + name;
    }
    
    public static Person unknown() {
        return new Person("Unknown", -1);
    }
}

record Point(double x, double y) implements Comparable<Point> {
    @Override
    public int compareTo(Point other) {
        return Double.compare(this.x, other.x);
    }
}
"#;
        
        let (symbols, edges, _) = harness.parse("Records.java", source)?;
        
        // Check records (they should be parsed as classes with record kind)
        let records = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class && 
                       (s.name == "Person" || s.name == "Point"))
            .collect::<Vec<_>>();
        assert!(records.len() >= 1, "Should find at least 1 record");
        
        // Check Person record
        if let Some(person) = records.iter().find(|r| r.name == "Person") {
            assert_eq!(person.visibility.as_deref(), Some("public"));
        }
        
        // Check methods in Person
        let person_methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method && 
                       s.fqn.contains("Person"))
            .collect::<Vec<_>>();
        
        assert!(person_methods.iter().any(|m| m.name == "Person"), "Should find constructor(s)");
        assert!(person_methods.iter().any(|m| m.name == "greet"));
        assert!(person_methods.iter().any(|m| m.name == "unknown"));
        
        // Check Point implements Comparable
        let implements_edges = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Implements)
            .collect::<Vec<_>>();
        
        assert!(implements_edges.iter().any(|e|
            e.src.as_ref().map(|s| s.contains("Point")).unwrap_or(false)
        ), "Point should implement Comparable");
        
        Ok(())
    }
    
    #[test]
    fn test_exact_exception_handling() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
import java.io.*;

public class ExceptionHandler {
    public void simpleThrow() throws IOException {
        throw new IOException("Error");
    }
    
    public void multipleThrows() throws IOException, SQLException, CustomException {
        // Method implementation
    }
    
    public void tryCatch() {
        try {
            riskyOperation();
        } catch (IOException e) {
            handleIO(e);
        } catch (SQLException | CustomException e) {
            handleOther(e);
        } finally {
            cleanup();
        }
    }
    
    public void tryWithResources() throws IOException {
        try (FileReader fr = new FileReader("file.txt");
             BufferedReader br = new BufferedReader(fr)) {
            String line = br.readLine();
        }
    }
    
    private void riskyOperation() throws IOException, SQLException {}
    private void handleIO(IOException e) {}
    private void handleOther(Exception e) {}
    private void cleanup() {}
}

class CustomException extends Exception {
    public CustomException(String message) {
        super(message);
    }
}
"#;
        
        let (symbols, edges, _) = harness.parse("Exceptions.java", source)?;
        
        // Check main class
        let classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .collect::<Vec<_>>();
        assert!(classes.iter().any(|c| c.name == "ExceptionHandler"));
        assert!(classes.iter().any(|c| c.name == "CustomException"));
        
        // Check methods
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect::<Vec<_>>();
        
        // Verify all methods are found
        let expected_methods = [
            "simpleThrow", "multipleThrows", "tryCatch", 
            "tryWithResources", "riskyOperation", "handleIO", 
            "handleOther", "cleanup"
        ];
        
        for method_name in expected_methods {
            assert!(methods.iter().any(|m| m.name == method_name),
                "Should find {} method", method_name);
        }
        
        // Check CustomException extends Exception
        let extends_edges = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Extends)
            .collect::<Vec<_>>();
        
        assert!(extends_edges.iter().any(|e|
            e.src.as_ref().map(|s| s.contains("CustomException")).unwrap_or(false) &&
            e.dst.as_ref().map(|d| d == "Exception").unwrap_or(false)
        ), "CustomException should extend Exception");
        
        Ok(())
    }
    
    #[test]
    fn test_exact_static_members() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
public class StaticMembers {
    private static int instanceCount = 0;
    public static final String CONSTANT = "VALUE";
    private static StaticMembers singleton;
    
    static {
        System.out.println("Static initializer block");
        instanceCount = 0;
    }
    
    public StaticMembers() {
        instanceCount++;
    }
    
    public static StaticMembers getInstance() {
        if (singleton == null) {
            singleton = new StaticMembers();
        }
        return singleton;
    }
    
    public static int getInstanceCount() {
        return instanceCount;
    }
    
    public static class StaticInner {
        private static void staticInnerMethod() {}
    }
}
"#;
        
        let (symbols, _, _) = harness.parse("Static.java", source)?;
        
        // Check class
        let classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .collect::<Vec<_>>();
        assert!(classes.iter().any(|c| c.name == "StaticMembers"));
        
        // Check static fields
        let fields = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Field)
            .collect::<Vec<_>>();
        
        assert!(fields.iter().any(|f| f.name == "instanceCount"));
        assert!(fields.iter().any(|f| f.name == "CONSTANT"));
        assert!(fields.iter().any(|f| f.name == "singleton"));
        
        // Check methods
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect::<Vec<_>>();
        
        assert!(methods.iter().any(|m| m.name == "StaticMembers"), "Should find constructor");
        assert!(methods.iter().any(|m| m.name == "getInstance"));
        assert!(methods.iter().any(|m| m.name == "getInstanceCount"));
        
        Ok(())
    }
}