import { User, UserService } from './user';

function createTestUser(id: number, name: string): User {
  return {
    id,
    name,
    email: `${name.toLowerCase()}@example.com`
  };
}

function main() {
  const service = new UserService();
  
  const user1 = createTestUser(1, 'Alice');
  const user2 = createTestUser(2, 'Bob');
  
  service.addUser(user1);
  service.addUser(user2);
  
  const found = service.findUser(1);
  if (found) {
    console.log(`Found user: ${found.name}`);
  }
  
  const allUsers = service.getAllUsers();
  console.log(`Total users: ${allUsers.length}`);
}

main();