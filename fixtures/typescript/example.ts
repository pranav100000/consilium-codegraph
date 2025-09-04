// TypeScript fixture with various language features

export interface IUser {
    id: number;
    name: string;
    email: string;
    isActive: boolean;
}

export abstract class BaseService<T> {
    protected items: T[] = [];
    
    abstract validate(item: T): boolean;
    
    add(item: T): void {
        if (this.validate(item)) {
            this.items.push(item);
        }
    }
    
    findById(id: number): T | undefined {
        return this.items.find((item: any) => item.id === id);
    }
}

export class UserService extends BaseService<IUser> {
    private emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    
    constructor(private readonly logger: Console = console) {
        super();
    }
    
    validate(user: IUser): boolean {
        return this.emailRegex.test(user.email) && user.name.length > 0;
    }
    
    async createUser(data: Omit<IUser, 'id'>): Promise<IUser> {
        const user: IUser = {
            ...data,
            id: this.generateId(),
        };
        
        this.add(user);
        this.logger.log('User created:', user);
        
        return user;
    }
    
    private generateId(): number {
        return Math.floor(Math.random() * 1000000);
    }
    
    getActiveUsers(): IUser[] {
        return this.items.filter(user => user.isActive);
    }
}

// Decorator example
function deprecated(message: string) {
    return function (target: any, propertyKey: string, descriptor: PropertyDescriptor) {
        const original = descriptor.value;
        
        descriptor.value = function (...args: any[]) {
            console.warn(`${propertyKey} is deprecated: ${message}`);
            return original.apply(this, args);
        };
        
        return descriptor;
    };
}

// Generic function with constraints
function merge<T extends object, U extends object>(obj1: T, obj2: U): T & U {
    return { ...obj1, ...obj2 };
}

// Type guards
function isUser(obj: any): obj is IUser {
    return obj &&
        typeof obj.id === 'number' &&
        typeof obj.name === 'string' &&
        typeof obj.email === 'string' &&
        typeof obj.isActive === 'boolean';
}

// Async iteration
async function* generateUsers(): AsyncGenerator<IUser> {
    const userService = new UserService();
    
    for (let i = 0; i < 5; i++) {
        const user = await userService.createUser({
            name: `User${i}`,
            email: `user${i}@example.com`,
            isActive: i % 2 === 0,
        });
        yield user;
    }
}

// Module augmentation
declare module "./example" {
    interface IUser {
        lastLogin?: Date;
    }
}

// Namespace
export namespace Utils {
    export function formatEmail(email: string): string {
        return email.toLowerCase().trim();
    }
    
    export function validateEmail(email: string): boolean {
        const regex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
        return regex.test(email);
    }
}

// Enum
export enum UserRole {
    Admin = 'ADMIN',
    User = 'USER',
    Guest = 'GUEST',
}

// Type alias
export type UserWithRole = IUser & { role: UserRole };

// Conditional types
type IsArray<T> = T extends any[] ? true : false;
type Test1 = IsArray<string[]>; // true
type Test2 = IsArray<number>; // false

// Template literal types
type EventName = `on${Capitalize<string>}`;
type ClickEvent = EventName; // "onClick", "onSubmit", etc.

// Main execution
async function main() {
    const service = new UserService();
    
    // Test user creation
    const user = await service.createUser({
        name: 'John Doe',
        email: 'john@example.com',
        isActive: true,
    });
    
    // Test user search
    const found = service.findById(user.id);
    if (found && isUser(found)) {
        console.log('Found user:', found.name);
    }
    
    // Test async iteration
    for await (const u of generateUsers()) {
        console.log('Generated user:', u.name);
    }
    
    // Test utility functions
    const formattedEmail = Utils.formatEmail('  JOHN@EXAMPLE.COM  ');
    console.log('Formatted email:', formattedEmail);
}

// Export default
export default main;