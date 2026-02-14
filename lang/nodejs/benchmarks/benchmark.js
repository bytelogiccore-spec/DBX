/**
 * DBX Native (napi-rs) vs better-sqlite3 - Performance Comparison
 */

const Database = require('../index.js').Database;
const BetterSqlite3 = require('better-sqlite3');

const N = 10000;

function benchmarkDBX() {
    console.log('Benchmarking DBX Native (napi-rs)...\n');

    const db = Database.openInMemory();

    // INSERT with transaction
    const startInsert = process.hrtime.bigint();
    const tx = db.beginTransaction();
    for (let i = 0; i < N; i++) {
        const key = Buffer.from(`key:${i}`);
        const value = Buffer.from(`value:${i}`);
        tx.insert('bench', key, value);
    }
    tx.commit();
    const endInsert = process.hrtime.bigint();
    const insertTime = Number(endInsert - startInsert) / 1e9;

    // GET
    const startGet = process.hrtime.bigint();
    for (let i = 0; i < N; i++) {
        const key = Buffer.from(`key:${i}`);
        db.get('bench', key);
    }
    const endGet = process.hrtime.bigint();
    const getTime = Number(endGet - startGet) / 1e9;

    // DELETE with transaction
    const startDelete = process.hrtime.bigint();
    const tx2 = db.beginTransaction();
    for (let i = 0; i < N; i++) {
        const key = Buffer.from(`key:${i}`);
        tx2.delete('bench', key);
    }
    tx2.commit();
    const endDelete = process.hrtime.bigint();
    const deleteTime = Number(endDelete - startDelete) / 1e9;

    db.close();

    return { insertTime, getTime, deleteTime };
}

function benchmarkSQLite() {
    console.log('Benchmarking better-sqlite3 (In-Memory)...\n');

    const db = new BetterSqlite3(':memory:');
    db.exec('CREATE TABLE bench (key BLOB PRIMARY KEY, value BLOB)');

    // INSERT with transaction
    const startInsert = process.hrtime.bigint();
    const insertStmt = db.prepare('INSERT INTO bench (key, value) VALUES (?, ?)');
    const insertMany = db.transaction((rows) => {
        for (const row of rows) {
            insertStmt.run(row.key, row.value);
        }
    });
    const rows = [];
    for (let i = 0; i < N; i++) {
        rows.push({
            key: Buffer.from(`key:${i}`),
            value: Buffer.from(`value:${i}`)
        });
    }
    insertMany(rows);
    const endInsert = process.hrtime.bigint();
    const insertTime = Number(endInsert - startInsert) / 1e9;

    // GET
    const startGet = process.hrtime.bigint();
    const getStmt = db.prepare('SELECT value FROM bench WHERE key = ?');
    for (let i = 0; i < N; i++) {
        const key = Buffer.from(`key:${i}`);
        getStmt.get(key);
    }
    const endGet = process.hrtime.bigint();
    const getTime = Number(endGet - startGet) / 1e9;

    // DELETE with transaction
    const startDelete = process.hrtime.bigint();
    const deleteStmt = db.prepare('DELETE FROM bench WHERE key = ?');
    const deleteMany = db.transaction((keys) => {
        for (const key of keys) {
            deleteStmt.run(key);
        }
    });
    const keys = [];
    for (let i = 0; i < N; i++) {
        keys.push(Buffer.from(`key:${i}`));
    }
    deleteMany(keys);
    const endDelete = process.hrtime.bigint();
    const deleteTime = Number(endDelete - startDelete) / 1e9;

    db.close();

    return { insertTime, getTime, deleteTime };
}

function printResults(name, { insertTime, getTime, deleteTime }) {
    console.log(`${name}:`);
    console.log(`  INSERT: ${insertTime.toFixed(4)}s (${Math.floor(N / insertTime).toLocaleString()} ops/sec)`);
    console.log(`  GET:    ${getTime.toFixed(4)}s (${Math.floor(N / getTime).toLocaleString()} ops/sec)`);
    console.log(`  DELETE: ${deleteTime.toFixed(4)}s (${Math.floor(N / deleteTime).toLocaleString()} ops/sec)`);
    console.log();
}

function main() {
    console.log('='.repeat(60));
    console.log('DBX Native (napi-rs) vs better-sqlite3 - Performance Comparison');
    console.log('='.repeat(60));
    console.log(`\nRunning benchmarks with ${N.toLocaleString()} operations...\n`);

    // Benchmark DBX
    const dbxResults = benchmarkDBX();
    printResults('DBX Native (napi-rs)', dbxResults);

    // Benchmark SQLite
    const sqliteResults = benchmarkSQLite();
    printResults('better-sqlite3 (In-Memory)', sqliteResults);

    // Comparison
    console.log('='.repeat(60));
    console.log('Performance Comparison:');
    console.log('='.repeat(60));

    const insertRatio = sqliteResults.insertTime / dbxResults.insertTime;
    const getRatio = sqliteResults.getTime / dbxResults.getTime;
    const deleteRatio = sqliteResults.deleteTime / dbxResults.deleteTime;

    if (insertRatio > 1) {
        console.log(`INSERT: DBX is ${insertRatio.toFixed(2)}x faster`);
    } else {
        console.log(`INSERT: SQLite is ${(1 / insertRatio).toFixed(2)}x faster`);
    }

    if (getRatio > 1) {
        console.log(`GET:    DBX is ${getRatio.toFixed(2)}x faster`);
    } else {
        console.log(`GET:    SQLite is ${(1 / getRatio).toFixed(2)}x faster`);
    }

    if (deleteRatio > 1) {
        console.log(`DELETE: DBX is ${deleteRatio.toFixed(2)}x faster`);
    } else {
        console.log(`DELETE: SQLite is ${(1 / deleteRatio).toFixed(2)}x faster`);
    }

    console.log('\n' + '='.repeat(60));
    console.log('Benchmark completed!');
    console.log('='.repeat(60));
}

main();
