const { Database } = require('../index.js');

console.log('DBX Native (napi-rs) - Basic Example\n');

// Open in-memory database
const db = Database.openInMemory();
console.log('✓ Opened in-memory database');

// Insert some data
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
db.insert('users', Buffer.from('user:2'), Buffer.from('Bob'));
console.log('✓ Inserted 2 users');

// Get data
const alice = db.get('users', Buffer.from('user:1'));
console.log(`✓ Retrieved: ${alice.toString()}`);

// Use transaction for batch operations
const tx = db.beginTransaction();
for (let i = 3; i <= 10; i++) {
    tx.insert('users', Buffer.from(`user:${i}`), Buffer.from(`User${i}`));
}
tx.commit();
console.log('✓ Inserted 8 more users via transaction');

// Delete data
db.delete('users', Buffer.from('user:1'));
console.log('✓ Deleted user:1');

// Close database
db.close();
console.log('✓ Closed database');

console.log('\n✅ All operations completed successfully!');
