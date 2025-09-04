#[cfg(test)]
pub mod fixtures {
    pub const SIMPLE_FUNCTION: &str = r#"
export function add(a: number, b: number): number {
    return a + b;
}

export const multiply = (x: number, y: number) => x * y;
"#;

    pub const CLASS_WITH_METHODS: &str = r#"
export class Calculator {
    private value: number = 0;
    
    add(n: number): void {
        this.value += n;
    }
    
    getValue(): number {
        return this.value;
    }
}
"#;

    pub const IMPORTS_EXAMPLE: &str = r#"
import { add } from './math';
import React from 'react';
import * as utils from '../utils';

export { add };
"#;

    pub const NESTED_FUNCTIONS: &str = r#"
function outer(x: number) {
    function inner(y: number) {
        return x + y;
    }
    return inner;
}
"#;

    pub const COMPLEX_GENERICS: &str = r#"
interface Container<T extends string | number, U = T> {
    value: T;
    transform<R>(fn: (val: T) => R): Container<R, U>;
}

class GenericClass<T extends { id: number }, K extends keyof T> {
    constructor(private item: T, private key: K) {}
    
    getValue(): T[K] {
        return this.item[this.key];
    }
}

type Conditional<T> = T extends string ? "string" : T extends number ? "number" : "other";
type Mapped<T> = { [K in keyof T]: T[K] | null };
"#;

    pub const DECORATORS_AND_METADATA: &str = r#"
@injectable()
@deprecated("Use NewService instead")
export class OldService {
    @readonly
    private static instance: OldService;
    
    @observable
    public count: number = 0;
    
    @memoize()
    @log("compute")
    compute(input: string): number {
        return input.length;
    }
}

@Component({
    selector: 'app-root',
    template: '<div>Hello</div>'
})
class AppComponent {}
"#;

    pub const ASYNC_AWAIT_PATTERNS: &str = r#"
async function fetchData<T>(url: string): Promise<T> {
    const response = await fetch(url);
    return await response.json() as T;
}

const asyncArrow = async () => {
    try {
        const [result1, result2] = await Promise.all([
            fetchData<User>('/api/user'),
            fetchData<Post[]>('/api/posts')
        ]);
        return { result1, result2 };
    } catch (error) {
        console.error(error);
        throw new Error('Failed to fetch');
    }
};

class AsyncClass {
    async *generator(): AsyncIterator<number> {
        for (let i = 0; i < 10; i++) {
            yield await Promise.resolve(i);
        }
    }
}
"#;

    pub const JSX_TSX_COMPONENTS: &str = r#"
import React, { useState, useEffect } from 'react';

interface Props {
    title: string;
    children?: React.ReactNode;
    onClick?: (event: React.MouseEvent) => void;
}

export const FunctionalComponent: React.FC<Props> = ({ title, children, onClick }) => {
    const [count, setCount] = useState<number>(0);
    
    useEffect(() => {
        console.log('Component mounted');
        return () => console.log('Component unmounted');
    }, []);
    
    return (
        <div className="container" onClick={onClick}>
            <h1>{title}</h1>
            <button onClick={() => setCount(c => c + 1)}>
                Count: {count}
            </button>
            {children}
        </div>
    );
};

class ClassComponent extends React.Component<Props, { value: string }> {
    render() {
        return <div>{this.props.title}</div>;
    }
}
"#;

    pub const NAMESPACE_AND_MODULES: &str = r#"
namespace MyNamespace {
    export interface Config {
        apiUrl: string;
        timeout: number;
    }
    
    export namespace Inner {
        export class Service {
            constructor(private config: Config) {}
        }
    }
    
    export function createService(config: Config): Inner.Service {
        return new Inner.Service(config);
    }
}

module MyModule {
    export const VERSION = "1.0.0";
    
    export module SubModule {
        export type Status = "active" | "inactive";
    }
}

declare module "external-lib" {
    export function externalFunc(): void;
}
"#;

    pub const ENUM_AND_CONST_ENUM: &str = r#"
enum Direction {
    Up = 1,
    Down,
    Left,
    Right
}

const enum FileAccess {
    None = 0,
    Read = 1 << 0,
    Write = 1 << 1,
    ReadWrite = Read | Write
}

enum StringEnum {
    A = "alpha",
    B = "beta",
    C = "gamma"
}

enum HeterogeneousEnum {
    No = 0,
    Yes = "YES"
}
"#;

    pub const ABSTRACT_AND_PROTECTED: &str = r#"
abstract class Animal {
    protected abstract makeSound(): string;
    protected name: string;
    
    constructor(name: string) {
        this.name = name;
    }
    
    public speak(): void {
        console.log(this.makeSound());
    }
}

class Dog extends Animal {
    private breed: string;
    
    constructor(name: string, breed: string) {
        super(name);
        this.breed = breed;
    }
    
    protected makeSound(): string {
        return "Woof!";
    }
    
    public getInfo(): string {
        return `${this.name} is a ${this.breed}`;
    }
}
"#;

    pub const PROPERTY_ACCESSORS: &str = r#"
class Person {
    private _age: number = 0;
    private _name: string;
    
    constructor(name: string) {
        this._name = name;
    }
    
    get name(): string {
        return this._name;
    }
    
    set name(value: string) {
        if (value.length > 0) {
            this._name = value;
        }
    }
    
    get age(): number {
        return this._age;
    }
    
    set age(value: number) {
        if (value >= 0 && value <= 150) {
            this._age = value;
        }
    }
    
    static get species(): string {
        return "Homo sapiens";
    }
}
"#;

    pub const COMPLEX_EXPORTS: &str = r#"
export { default } from './module';
export * from './types';
export * as utils from './utils';
export { foo as bar, baz } from './items';

const value1 = 1;
const value2 = 2;

export { value1, value2 };

export = {
    method1() {},
    method2() {}
};

export as namespace MyLib;
"#;

    pub const TYPE_GUARDS_AND_ASSERTIONS: &str = r#"
function isString(value: unknown): value is string {
    return typeof value === 'string';
}

function isArray<T>(value: T | T[]): value is T[] {
    return Array.isArray(value);
}

interface Cat {
    meow(): void;
}

interface Dog {
    bark(): void;
}

function isCat(pet: Cat | Dog): pet is Cat {
    return 'meow' in pet;
}

function processValue(value: unknown) {
    if (isString(value)) {
        console.log(value.toUpperCase());
    }
    
    const str = value as string;
    const len = (<string>value).length;
    const num = value as unknown as number;
}
"#;

    pub const UNICODE_AND_SPECIAL_CHARS: &str = r#"
// Unicode identifiers
const 你好 = "hello";
const مرحبا = "hello";
const 🚀rocket = "fast";

function 计算(参数1: number, 参数2: number): number {
    return 参数1 + 参数2;
}

class КлассПример {
    приватноеПоле: string = "тест";
    
    публичныйМетод(): void {
        console.log(this.приватноеПоле);
    }
}

// Special characters in strings and templates
const special = "Line 1\nLine 2\tTabbed\r\nWindows line";
const template = `Unicode: \u{1F600} Path: C:\\Users\\Name`;
const regex = /[\x00-\x1F\x7F]/g;
"#;

    pub const MALFORMED_CODE: &str = r#"
// Missing closing brace
function broken(x: number {
    return x * 2;

// Unclosed string
const str = "hello world

// Invalid syntax
class {
    method() {}
}

// Missing type after colon
let value: = 5;

// Incomplete generic
interface Container<> {
    value: T;
}
"#;

    pub const EMPTY_FILE: &str = "";

    pub const ONLY_COMMENTS: &str = r#"
// This file contains only comments
/* Multi-line comment
   spanning several lines
   with no actual code */

// Another comment

/**
 * JSDoc comment
 * @param {string} param - Description
 * @returns {number} - Return description
 */

// TODO: Add implementation
// FIXME: Fix this issue
// NOTE: Important note
"#;

    pub const LARGE_FILE: &str = {
        // Generate a large file with many symbols
        const BASE: &str = r#"
export function func$ID$(param: string): number {
    return param.length + $ID$;
}

export class Class$ID$ {
    private field$ID$: number = $ID$;
    
    method$ID$(): void {
        console.log(this.field$ID$);
    }
}

export interface Interface$ID$ {
    prop$ID$: string;
    method$ID$(): number;
}

export type Type$ID$ = string | number | Interface$ID$;

"#;
        
        // This is a compile-time constant, so we can't generate it dynamically
        // Instead, we'll use a smaller representative sample
        r#"
export function func1(param: string): number { return param.length + 1; }
export function func2(param: string): number { return param.length + 2; }
export function func3(param: string): number { return param.length + 3; }
export function func4(param: string): number { return param.length + 4; }
export function func5(param: string): number { return param.length + 5; }

export class Class1 { private field1: number = 1; method1(): void { console.log(this.field1); } }
export class Class2 { private field2: number = 2; method2(): void { console.log(this.field2); } }
export class Class3 { private field3: number = 3; method3(): void { console.log(this.field3); } }
export class Class4 { private field4: number = 4; method4(): void { console.log(this.field4); } }
export class Class5 { private field5: number = 5; method5(): void { console.log(this.field5); } }

export interface Interface1 { prop1: string; method1(): number; }
export interface Interface2 { prop2: string; method2(): number; }
export interface Interface3 { prop3: string; method3(): number; }
export interface Interface4 { prop4: string; method4(): number; }
export interface Interface5 { prop5: string; method5(): number; }

export type Type1 = string | number | Interface1;
export type Type2 = string | number | Interface2;
export type Type3 = string | number | Interface3;
export type Type4 = string | number | Interface4;
export type Type5 = string | number | Interface5;
"#
    };

    pub const INDEX_SIGNATURES: &str = r#"
interface StringArray {
    [index: number]: string;
}

interface Dictionary<T> {
    [key: string]: T;
}

interface ReadonlyDict {
    readonly [key: string]: any;
}

class FlexibleClass {
    [key: string]: any;
    
    knownProp: string = "known";
    
    constructor() {
        this["dynamicProp"] = 42;
    }
}
"#;

    pub const TRIPLE_SLASH_DIRECTIVES: &str = r#"
/// <reference path="./types.d.ts" />
/// <reference types="node" />
/// <reference lib="es2020" />

import * as fs from 'fs';

/// <amd-module name="MyModule" />
/// <amd-dependency path="legacy/module" />

export function readFile(path: string): string {
    return fs.readFileSync(path, 'utf8');
}
"#;

    pub const INTERSECTION_AND_UNION_TYPES: &str = r#"
type Person = {
    name: string;
    age: number;
};

type Employee = {
    employeeId: string;
    department: string;
};

type Manager = Person & Employee & {
    reports: Employee[];
};

type StringOrNumber = string | number;
type Nullable<T> = T | null | undefined;
type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

function processValue(value: string | number | boolean | null): void {
    if (typeof value === 'string') {
        console.log(value.toUpperCase());
    } else if (typeof value === 'number') {
        console.log(value * 2);
    } else if (value === null) {
        console.log('null value');
    } else {
        console.log('boolean:', value);
    }
}
"#;
}