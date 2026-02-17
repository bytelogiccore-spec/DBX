/**
 * FFI Performance Benchmark for DBX Node.js Bindings
 * 
 * Measures INSERT, GET, and SCAN performance to quantify FFI overhead
 * compared to Rust Core benchmarks.
 * 
 * Run with: node benchmarks/ffi_benchmark.js
 */

const Benchmark = require('benchmark');
const { Database } = require('../');

const NUM_ENTRIES = 10_000;

function generateTestData() {
    const data = [];
    for (let i = 0; i < NUM_ENTRIES; i++) {
        const key = Buffer.from(`key_${i.toString().padStart(8, '0')}`);
        const value = Buffer.from(`value_${i.toString().padStart(8, '0')}_data`);
        data.push([key, value]);
    }
    return data;
}

// Prepare data for GET and SCAN benchmarks
let getDb, scanDb, testData;

const suite = new Benchmark.Suite();

suite
    .add('DBX INSERT 10k', function () {
        const db = Database.openInMemory();
        const data = generateTestData();
        for (const [key, value] of data) {
            db.insert('bench', key, value);
        }
    })
    .add('DBX GET 10k', {
        onStart: function () {
            getDb = Database.openInMemory();
            testData = generateTestData();
            for (const [key, value] of testData) {
                getDb.insert('bench', key, value);
            }
        },
        fn: function () {
            for (const [key] of testData) {
                getDb.get('bench', key);
            }
        }
    })
    .add('DBX SCAN 10k', {
        onStart: function () {
            scanDb = Database.openInMemory();
            const data = generateTestData();
            for (const [key, value] of data) {
                scanDb.insert('bench', key, value);
            }
        },
        fn: function () {
            scanDb.scan('bench');
        }
    })
    .on('cycle', function (event) {
        console.log(String(event.target));
        const timeMs = event.target.stats.mean * 1000;
        console.log(`  → ${timeMs.toFixed(2)}ms per operation\n`);
    })
    .on('complete', function () {
        console.log('\n=== Benchmark Complete ===');
    })
    .run({ 'async': false });
