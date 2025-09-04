  import { User, UserService } from './user';
// definition scip-typescript npm . . `main.ts`/
//documentation
//> ```ts
//> module "main.ts"
//> ```
//         ^^^^ reference scip-typescript npm . . `user.ts`/User#
//               ^^^^^^^^^^^ reference scip-typescript npm . . `user.ts`/UserService#
//                                  ^^^^^^^^ reference scip-typescript npm . . `user.ts`/
  
  function createTestUser(id: number, name: string): User {
//         ^^^^^^^^^^^^^^ definition scip-typescript npm . . `main.ts`/createTestUser().
//         documentation
//         > ```ts
//         > function createTestUser(id: number, name: string): User
//         > ```
//                        ^^ definition scip-typescript npm . . `main.ts`/createTestUser().(id)
//                        documentation
//                        > ```ts
//                        > (parameter) id: number
//                        > ```
//                                    ^^^^ definition scip-typescript npm . . `main.ts`/createTestUser().(name)
//                                    documentation
//                                    > ```ts
//                                    > (parameter) name: string
//                                    > ```
//                                                   ^^^^ reference scip-typescript npm . . `user.ts`/User#
    return {
      id,
//    ^^ reference scip-typescript npm . . `user.ts`/User#id.
      name,
//    ^^^^ reference scip-typescript npm . . `user.ts`/User#name.
      email: `${name.toLowerCase()}@example.com`
//    ^^^^^ reference scip-typescript npm . . `user.ts`/User#email.
//              ^^^^ reference scip-typescript npm . . `main.ts`/createTestUser().(name)
//                   ^^^^^^^^^^^ reference scip-typescript npm typescript 5.9.2 lib/`lib.es5.d.ts`/String#toLowerCase().
    };
  }
  
  function main() {
//         ^^^^ definition scip-typescript npm . . `main.ts`/main().
//         documentation
//         > ```ts
//         > function main(): void
//         > ```
    const service = new UserService();
//        ^^^^^^^ definition local 2
//        documentation
//        > ```ts
//        > var service: UserService
//        > ```
//                      ^^^^^^^^^^^ reference scip-typescript npm . . `user.ts`/UserService#
    
    const user1 = createTestUser(1, 'Alice');
//        ^^^^^ definition local 5
//        documentation
//        > ```ts
//        > var user1: User
//        > ```
//                ^^^^^^^^^^^^^^ reference scip-typescript npm . . `main.ts`/createTestUser().
    const user2 = createTestUser(2, 'Bob');
//        ^^^^^ definition local 8
//        documentation
//        > ```ts
//        > var user2: User
//        > ```
//                ^^^^^^^^^^^^^^ reference scip-typescript npm . . `main.ts`/createTestUser().
    
    service.addUser(user1);
//  ^^^^^^^ reference local 2
//          ^^^^^^^ reference scip-typescript npm . . `user.ts`/UserService#addUser().
//                  ^^^^^ reference local 5
    service.addUser(user2);
//  ^^^^^^^ reference local 2
//          ^^^^^^^ reference scip-typescript npm . . `user.ts`/UserService#addUser().
//                  ^^^^^ reference local 8
    
    const found = service.findUser(1);
//        ^^^^^ definition local 11
//        documentation
//        > ```ts
//        > var found: User | undefined
//        > ```
//                ^^^^^^^ reference local 2
//                        ^^^^^^^^ reference scip-typescript npm . . `user.ts`/UserService#findUser().
    if (found) {
//      ^^^^^ reference local 11
      console.log(`Found user: ${found.name}`);
//                               ^^^^^ reference local 11
//                                     ^^^^ reference scip-typescript npm . . `user.ts`/User#name.
    }
    
    const allUsers = service.getAllUsers();
//        ^^^^^^^^ definition local 14
//        documentation
//        > ```ts
//        > var allUsers: User[]
//        > ```
//                   ^^^^^^^ reference local 2
//                           ^^^^^^^^^^^ reference scip-typescript npm . . `user.ts`/UserService#getAllUsers().
    console.log(`Total users: ${allUsers.length}`);
//                              ^^^^^^^^ reference local 14
//                                       ^^^^^^ reference scip-typescript npm typescript 5.9.2 lib/`lib.es5.d.ts`/Array#length.
  }
  
  main();
//^^^^ reference scip-typescript npm . . `main.ts`/main().
