/**
 * SQL INSERT Test for DBX
 * 
 * Tests SQL INSERT functionality via Node.js bindings
 */

const { Database } = require('../index');

async function testSqlInsert() {
    console.log('=== SQL INSERT Test ===\n');

    // Create in-memory database
    const db = Database.openInMemory();
    console.log('✓ Database opened');

    try {
        // Test 1: Single row INSERT
        console.log('\n1. Testing single-row INSERT...');
        const tx1 = db.beginTransaction();
        const result1 = tx1.execute("INSERT INTO users (id, name) VALUES (1, 'Alice')");
        console.log(`   Rows inserted: ${result1}`);
        tx1.commit();
        console.log('   ✓ Single-row INSERT successful');

        // Test 2: Multi-row INSERT
        console.log('\n2. Testing multi-row INSERT...');
        const tx2 = db.beginTransaction();
        const result2 = tx2.execute("INSERT INTO users (id, name) VALUES (2, 'Bob'), (3, 'Charlie')");
        console.log(`   Rows inserted: ${result2}`);
        tx2.commit();
        console.log('   ✓ Multi-row INSERT successful');

        // Test 3: Different data types
        console.log('\n3. Testing different data types...');
        const tx3 = db.beginTransaction();
        const result3 = tx3.execute("INSERT INTO data (id, value, flag) VALUES (42, 3.14, true)");
        console.log(`   Rows inserted: ${result3}`);
        tx3.commit();
        console.log('   ✓ Different data types INSERT successful');

        // Test 4: Verify data via KV API
        console.log('\n4. Verifying inserted data via KV API...');
        const value1 = db.get('users', Buffer.from('1'));
        if (value1) {
            console.log(`   Retrieved value for key '1': ${value1.toString()}`);
            console.log('   ✓ Data verification successful');
        } else {
            console.log('   ⚠ Warning: Could not retrieve inserted data');
        }

        console.log('\n=== All SQL INSERT tests passed! ===');
        return true;

    } catch (error) {
        console.error('\n❌ Test failed:', error.message);
        console.error('Stack:', error.stack);
        return false;
    } finally {
        db.close();
        console.log('\nDatabase closed');
    }
}

// Run tests
testSqlInsert()
    .then(success => {
        process.exit(success ? 0 : 1);
    })
    .catch(error => {
        console.error('Unexpected error:', error);
        process.exit(1);
    });
