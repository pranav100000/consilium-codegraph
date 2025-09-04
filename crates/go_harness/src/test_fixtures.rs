#[cfg(test)]
pub mod fixtures {
    pub const SIMPLE_FUNCTION: &str = r#"
package main

func add(a int, b int) int {
    return a + b
}

func multiply(x, y int) int {
    return x * y
}
"#;

    pub const STRUCT_WITH_METHODS: &str = r#"
package main

type User struct {
    Name  string
    Email string
    Age   int
}

func (u *User) GetName() string {
    return u.Name
}

func (u User) IsAdult() bool {
    return u.Age >= 18
}
"#;

    pub const COMPLEX_TYPES: &str = r#"
package main

import (
    "context"
    "sync"
)

// Generic types (Go 1.18+)
type Stack[T any] struct {
    items []T
    mu    sync.Mutex
}

func (s *Stack[T]) Push(item T) {
    s.mu.Lock()
    defer s.mu.Unlock()
    s.items = append(s.items, item)
}

func (s *Stack[T]) Pop() (T, bool) {
    s.mu.Lock()
    defer s.mu.Unlock()
    
    var zero T
    if len(s.items) == 0 {
        return zero, false
    }
    
    item := s.items[len(s.items)-1]
    s.items = s.items[:len(s.items)-1]
    return item, true
}

// Type constraints
type Number interface {
    ~int | ~int8 | ~int16 | ~int32 | ~int64 |
    ~uint | ~uint8 | ~uint16 | ~uint32 | ~uint64 |
    ~float32 | ~float64
}

func Sum[T Number](values []T) T {
    var sum T
    for _, v := range values {
        sum += v
    }
    return sum
}

// Complex nested types
type Config struct {
    Server struct {
        Host string
        Port int
        TLS  *struct {
            Cert string
            Key  string
        }
    }
    Database map[string]struct {
        Host     string
        Port     int
        Username string
        Password string
    }
    Features []string
}
"#;

    pub const INTERFACES_AND_EMBEDDING: &str = r#"
package main

import "io"

// Basic interface
type Shape interface {
    Area() float64
    Perimeter() float64
}

// Interface embedding
type ReadWriteCloser interface {
    io.Reader
    io.Writer
    io.Closer
}

// Interface with type constraints
type Comparable[T any] interface {
    Compare(other T) int
}

// Struct embedding
type Person struct {
    Name string
    Age  int
}

type Employee struct {
    Person // Embedded struct
    ID     string
    Salary float64
}

// Method on embedded type
func (p Person) String() string {
    return p.Name
}

// Interface implementation
type Circle struct {
    Radius float64
}

func (c Circle) Area() float64 {
    return 3.14159 * c.Radius * c.Radius
}

func (c Circle) Perimeter() float64 {
    return 2 * 3.14159 * c.Radius
}

// Type alias
type MyInt = int
type StringList = []string

// Named type
type UserID int
type Temperature float64
"#;

    pub const GOROUTINES_AND_CHANNELS: &str = r#"
package main

import (
    "context"
    "sync"
    "time"
)

// Channel operations
func producer(ch chan<- int) {
    for i := 0; i < 10; i++ {
        ch <- i
    }
    close(ch)
}

func consumer(ch <-chan int, done chan<- bool) {
    for value := range ch {
        println(value)
    }
    done <- true
}

// Select statement
func worker(ctx context.Context, jobs <-chan int, results chan<- int) {
    for {
        select {
        case job, ok := <-jobs:
            if !ok {
                return
            }
            results <- job * 2
        case <-ctx.Done():
            return
        case <-time.After(1 * time.Second):
            println("timeout")
        }
    }
}

// WaitGroup and Mutex
type Counter struct {
    mu    sync.RWMutex
    value int
}

func (c *Counter) Increment() {
    c.mu.Lock()
    defer c.mu.Unlock()
    c.value++
}

func (c *Counter) Value() int {
    c.mu.RLock()
    defer c.mu.RUnlock()
    return c.value
}

// Goroutine with closure
func startWorkers(n int) {
    var wg sync.WaitGroup
    
    for i := 0; i < n; i++ {
        wg.Add(1)
        go func(id int) {
            defer wg.Done()
            println("Worker", id)
        }(i)
    }
    
    wg.Wait()
}
"#;

    pub const ERROR_HANDLING: &str = r#"
package main

import (
    "errors"
    "fmt"
)

// Custom error type
type ValidationError struct {
    Field   string
    Message string
}

func (e ValidationError) Error() string {
    return fmt.Sprintf("validation error on field %s: %s", e.Field, e.Message)
}

// Error wrapping
func processData(data []byte) error {
    if len(data) == 0 {
        return errors.New("empty data")
    }
    
    if err := validate(data); err != nil {
        return fmt.Errorf("validation failed: %w", err)
    }
    
    return nil
}

// Multiple return with error
func divide(a, b float64) (float64, error) {
    if b == 0 {
        return 0, errors.New("division by zero")
    }
    return a / b, nil
}

// Panic and recover
func safeDivide(a, b float64) (result float64, err error) {
    defer func() {
        if r := recover(); r != nil {
            err = fmt.Errorf("panic recovered: %v", r)
        }
    }()
    
    if b == 0 {
        panic("division by zero")
    }
    
    return a / b, nil
}

// Error sentinel
var (
    ErrNotFound = errors.New("not found")
    ErrInvalid  = errors.New("invalid input")
)

func findUser(id string) (*User, error) {
    if id == "" {
        return nil, ErrInvalid
    }
    // ... lookup logic
    return nil, ErrNotFound
}
"#;

    pub const REFLECTION_AND_TAGS: &str = r#"
package main

import (
    "reflect"
    "encoding/json"
    "errors"
    "time"
)

// Struct with tags
type User struct {
    ID        int       ` + "`" + `json:"id" db:"user_id"` + "`" + `
    Name      string    ` + "`" + `json:"name,omitempty" validate:"required,min=3,max=50"` + "`" + `
    Email     string    ` + "`" + `json:"email" validate:"required,email"` + "`" + `
    Password  string    ` + "`" + `json:"-" db:"password_hash"` + "`" + `
    CreatedAt time.Time ` + "`" + `json:"created_at" db:"created_at"` + "`" + `
    Metadata  map[string]interface{} ` + "`" + `json:"metadata,omitempty"` + "`" + `
}

// Using reflection
func PrintStructFields(v interface{}) {
    t := reflect.TypeOf(v)
    val := reflect.ValueOf(v)
    
    for i := 0; i < t.NumField(); i++ {
        field := t.Field(i)
        value := val.Field(i)
        
        jsonTag := field.Tag.Get("json")
        dbTag := field.Tag.Get("db")
        
        println(field.Name, value.Interface(), jsonTag, dbTag)
    }
}

// Method with reflection
func CallMethod(obj interface{}, methodName string, args ...interface{}) ([]reflect.Value, error) {
    method := reflect.ValueOf(obj).MethodByName(methodName)
    if !method.IsValid() {
        return nil, errors.New("method not found")
    }
    
    in := make([]reflect.Value, len(args))
    for i, arg := range args {
        in[i] = reflect.ValueOf(arg)
    }
    
    return method.Call(in), nil
}
"#;

    pub const INIT_AND_PACKAGES: &str = r#"
package mypackage

import (
    "fmt"
    "sync"
)

// Package-level variables
var (
    instance *Singleton
    once     sync.Once
    
    globalCounter int
    mutex         sync.Mutex
)

// Constants
const (
    MaxRetries = 3
    Timeout    = 30
    
    StatusOK    = 200
    StatusError = 500
)

// Iota
const (
    Monday = iota
    Tuesday
    Wednesday
    Thursday
    Friday
    Saturday
    Sunday
)

// init function
func init() {
    fmt.Println("Package initialized")
    globalCounter = 0
}

// Another init function
func init() {
    // Multiple init functions are allowed
    instance = &Singleton{}
}

// Singleton pattern
type Singleton struct {
    data string
}

func GetInstance() *Singleton {
    once.Do(func() {
        instance = &Singleton{
            data: "initialized",
        }
    })
    return instance
}

// Exported and unexported
func PublicFunction() string {
    return privateHelper()
}

func privateHelper() string {
    return "helper"
}

type PublicStruct struct {
    PublicField  string
    privateField string
}

type privateStruct struct {
    field string
}
"#;

    pub const CLOSURES_AND_FUNCTIONS: &str = r#"
package main

import "fmt"

// Function as parameter
func apply(fn func(int) int, value int) int {
    return fn(value)
}

// Function returning function
func makeMultiplier(factor int) func(int) int {
    return func(x int) int {
        return x * factor
    }
}

// Variadic function
func sum(nums ...int) int {
    total := 0
    for _, n := range nums {
        total += n
    }
    return total
}

// Named return values
func divmod(a, b int) (quotient, remainder int) {
    quotient = a / b
    remainder = a % b
    return // naked return
}

// Closure capturing variables
func counter() func() int {
    count := 0
    return func() int {
        count++
        return count
    }
}

// Anonymous function
var add = func(a, b int) int {
    return a + b
}

// Function type
type Operation func(int, int) int

func calculate(op Operation, a, b int) int {
    return op(a, b)
}

// Method expression and value
type Calculator struct {
    value int
}

func (c *Calculator) Add(n int) {
    c.value += n
}

func useMethodExpression() {
    var fn func(*Calculator, int) = (*Calculator).Add
    calc := &Calculator{}
    fn(calc, 5)
}

func useMethodValue() {
    calc := &Calculator{}
    fn := calc.Add
    fn(5)
}
"#;

    pub const TESTING_AND_BENCHMARKS: &str = r#"
package main

import (
    "testing"
    "benchmark"
)

// Test function
func TestAdd(t *testing.T) {
    result := Add(2, 3)
    if result != 5 {
        t.Errorf("Add(2, 3) = %d; want 5", result)
    }
}

// Table-driven test
func TestMultiply(t *testing.T) {
    tests := []struct {
        name string
        a, b int
        want int
    }{
        {"positive", 2, 3, 6},
        {"negative", -2, 3, -6},
        {"zero", 0, 5, 0},
    }
    
    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            if got := Multiply(tt.a, tt.b); got != tt.want {
                t.Errorf("Multiply(%d, %d) = %d; want %d", tt.a, tt.b, got, tt.want)
            }
        })
    }
}

// Benchmark
func BenchmarkAdd(b *testing.B) {
    for i := 0; i < b.N; i++ {
        Add(2, 3)
    }
}

// Parallel benchmark
func BenchmarkParallel(b *testing.B) {
    b.RunParallel(func(pb *testing.PB) {
        for pb.Next() {
            Add(2, 3)
        }
    })
}

// Example test
func ExampleAdd() {
    fmt.Println(Add(2, 3))
    // Output: 5
}

// Fuzzing (Go 1.18+)
func FuzzAdd(f *testing.F) {
    f.Add(2, 3)
    f.Add(-1, 1)
    f.Add(0, 0)
    
    f.Fuzz(func(t *testing.T, a, b int) {
        result := Add(a, b)
        if result != a+b {
            t.Errorf("Add(%d, %d) = %d; want %d", a, b, result, a+b)
        }
    })
}
"#;

    pub const UNICODE_AND_SPECIAL_NAMES: &str = r#"
package main

// Unicode identifiers
var 你好 = "hello"
var مرحبا = "hello"
var π = 3.14159

func 计算(参数1, 参数2 int) int {
    return 参数1 + 参数2
}

type 用户 struct {
    名字 string
    年龄 int
}

func (u *用户) 获取名字() string {
    return u.名字
}

// Special characters in strings
const (
    newline = "Line 1\nLine 2"
    tab     = "Col1\tCol2"
    quote   = "He said \"Hello\""
    backtick = ` + "`" + `This is a raw string with "quotes" and \backslashes\` + "`" + `
    unicode = "\u4e16\u754c" // 世界
)

// Rune literals
const (
    letterA = 'A'
    smiley  = '😀'
    chinese = '中'
)
"#;

    pub const UNSAFE_AND_CGO: &str = r#"
package main

/*
#include <stdio.h>
#include <stdlib.h>

void hello() {
    printf("Hello from C!\n");
}
*/
import "C"
import (
    "unsafe"
)

// Using unsafe
func unsafeConversion() {
    var i int = 42
    var p *int = &i
    
    // Convert to unsafe.Pointer
    up := unsafe.Pointer(p)
    
    // Convert to different type
    var fp *float64 = (*float64)(up)
    
    // Size and offset
    size := unsafe.Sizeof(i)
    align := unsafe.Alignof(i)
    
    type S struct {
        a int
        b string
    }
    offset := unsafe.Offsetof(S{}.b)
}

// CGo function
func callC() {
    C.hello()
    
    // C string
    cs := C.CString("Hello")
    defer C.free(unsafe.Pointer(cs))
}

// Assembly declaration
//go:noescape
func add(a, b int) int

// Linkname directive
//go:linkname privateFunc runtime.privateFunc
func privateFunc()
"#;

    pub const BUILD_TAGS: &str = r#"
//go:build linux && amd64
// +build linux,amd64

package main

// Build constraints
//go:build (linux || darwin) && !windows
// +build linux darwin
// +build !windows

import "fmt"

// Conditional compilation
func platformSpecific() {
    fmt.Println("Linux or Darwin, AMD64")
}

// Generate directive
//go:generate go run gen.go

// Embed directive (Go 1.16+)
import _ "embed"

//go:embed resources/config.json
var configData []byte

//go:embed templates/*
var templates embed.FS
"#;

    pub const MALFORMED_CODE: &str = r#"
package main

// Missing closing brace
func broken(x int {
    return x * 2

// Unclosed string
var str = "hello world

// Invalid syntax
type {
    field int
}

// Missing type
func noType(x) {
    return x
}

// Incomplete struct
type Incomplete struct {
    field1 int
    field2

// Invalid import
import fmt

// Syntax error
func() {
    return
}
"#;

    pub const EMPTY_FILE: &str = "";

    pub const ONLY_COMMENTS: &str = r#"
// This file contains only comments
/* Multi-line comment
   spanning several lines
   with no actual code */

// Another comment

// TODO: Add implementation
// FIXME: Fix this issue
// NOTE: Important note
// BUG(user): Known bug

/*
Package documentation would go here
but there's no package declaration
*/
"#;

    pub const LARGE_FILE: &str = r#"
package main

func func1(p int) int { return p + 1 }
func func2(p int) int { return p + 2 }
func func3(p int) int { return p + 3 }
func func4(p int) int { return p + 4 }
func func5(p int) int { return p + 5 }

type Type1 struct { Field1 int }
type Type2 struct { Field2 int }
type Type3 struct { Field3 int }
type Type4 struct { Field4 int }
type Type5 struct { Field5 int }

func (t *Type1) Method1() {}
func (t *Type2) Method2() {}
func (t *Type3) Method3() {}
func (t *Type4) Method4() {}
func (t *Type5) Method5() {}

var var1 = 1
var var2 = 2
var var3 = 3
var var4 = 4
var var5 = 5

const const1 = 1
const const2 = 2
const const3 = 3
const const4 = 4
const const5 = 5

type Interface1 interface { Method1() }
type Interface2 interface { Method2() }
type Interface3 interface { Method3() }
type Interface4 interface { Method4() }
type Interface5 interface { Method5() }
"#;
}