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
}