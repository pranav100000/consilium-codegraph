  export interface User {
// definition scip-typescript npm . . `user.ts`/
//documentation
//> ```ts
//> module "user.ts"
//> ```
//                 ^^^^ definition scip-typescript npm . . `user.ts`/User#
//                 documentation
//                 > ```ts
//                 > interface User
//                 > ```
    id: number;
//  ^^ definition scip-typescript npm . . `user.ts`/User#id.
//  documentation
//  > ```ts
//  > (property) id: number
//  > ```
    name: string;
//  ^^^^ definition scip-typescript npm . . `user.ts`/User#name.
//  documentation
//  > ```ts
//  > (property) name: string
//  > ```
    email: string;
//  ^^^^^ definition scip-typescript npm . . `user.ts`/User#email.
//  documentation
//  > ```ts
//  > (property) email: string
//  > ```
  }
  
  export class UserService {
//             ^^^^^^^^^^^ definition scip-typescript npm . . `user.ts`/UserService#
//             documentation
//             > ```ts
//             > class UserService
//             > ```
    private users: User[] = [];
//          ^^^^^ definition scip-typescript npm . . `user.ts`/UserService#users.
//          documentation
//          > ```ts
//          > (property) users: User[]
//          > ```
//                 ^^^^ reference scip-typescript npm . . `user.ts`/User#
  
    addUser(user: User): void {
//  ^^^^^^^ definition scip-typescript npm . . `user.ts`/UserService#addUser().
//  documentation
//  > ```ts
//  > (method) addUser(user: User): void
//  > ```
//          ^^^^ definition scip-typescript npm . . `user.ts`/UserService#addUser().(user)
//          documentation
//          > ```ts
//          > (parameter) user: User
//          > ```
//                ^^^^ reference scip-typescript npm . . `user.ts`/User#
      this.users.push(user);
//         ^^^^^ reference scip-typescript npm . . `user.ts`/UserService#users.
//               ^^^^ reference scip-typescript npm typescript 5.9.2 lib/`lib.es5.d.ts`/Array#push().
//                    ^^^^ reference scip-typescript npm . . `user.ts`/UserService#addUser().(user)
    }
  
    findUser(id: number): User | undefined {
//  ^^^^^^^^ definition scip-typescript npm . . `user.ts`/UserService#findUser().
//  documentation
//  > ```ts
//  > (method) findUser(id: number): User | undefined
//  > ```
//           ^^ definition scip-typescript npm . . `user.ts`/UserService#findUser().(id)
//           documentation
//           > ```ts
//           > (parameter) id: number
//           > ```
//                        ^^^^ reference scip-typescript npm . . `user.ts`/User#
      return this.users.find(u => u.id === id);
//                ^^^^^ reference scip-typescript npm . . `user.ts`/UserService#users.
//                      ^^^^ reference scip-typescript npm typescript 5.9.2 lib/`lib.es2015.core.d.ts`/Array#find().
//                           ^ definition local 3
//                           documentation
//                           > ```ts
//                           > (parameter) u: User
//                           > ```
//                                ^ reference local 3
//                                  ^^ reference scip-typescript npm . . `user.ts`/User#id.
//                                         ^^ reference scip-typescript npm . . `user.ts`/UserService#findUser().(id)
    }
  
    getAllUsers(): User[] {
//  ^^^^^^^^^^^ definition scip-typescript npm . . `user.ts`/UserService#getAllUsers().
//  documentation
//  > ```ts
//  > (method) getAllUsers(): User[]
//  > ```
//                 ^^^^ reference scip-typescript npm . . `user.ts`/User#
      return this.users;
//                ^^^^^ reference scip-typescript npm . . `user.ts`/UserService#users.
    }
  }
