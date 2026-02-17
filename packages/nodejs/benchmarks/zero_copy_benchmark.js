/**
 * Zero-Copy FFI Performance Benchmark for DBX Node.js Bindings
 * 
 * Compares standard API vs zero-copy API performance
 * 
 * Run with: node benchmarks/zero_copy_benchmark.js
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

// Prepare databases
let standardDb, zeroCopyDb, batchDb;
let testData, testKeys;

const suite = new Benchmark.Suite();

suite
    .add('Standard SCAN 10k', {
        onStart: function () {
            standardDb = Database.openInMemory();
            const data = generateTestData();
            for (const [key, value] of data) {
                standardDb.insert('bench', key, value);
            }
        },
        fn: function () {
            standardDb.scan('bench');
        }
    })
    .add('Zero-Copy SCAN 10k', {
        onStart: function () {
            zeroCopyDb = Database.openInMemory();
            const data = generateTestData();
            for (const [key, value] of data) {
                zeroCopyDb.insert('bench', key, value);
            }
        },
        fn: function () {
            const result = zeroCopyDb.scanZeroCopy('bench');
            // Access the buffer (zero-copy)
            const buffer = result.asBuffer();
            const count = result.count();
        }
    })
    .add('Standard GET 10k', {
        onStart: function () {
            standardDb = Database.openInMemory();
            testData = generateTestData();
            testKeys = testData.map(([key]) => key);
            for (const [key, value] of testData) {
                standardDb.insert('bench', key, value);
            }
        },
        fn: function () {
            for (const key of testKeys) {
                standardDb.get('bench', key);
            }
        }
    })
    .add('Batch GET 10k', {
        onStart: function () {
            batchDb = Database.openInMemory();
            testData = generateTestData();
            testKeys = testData.map(([key]) => key);
            for (const [key, value] of testData) {
                batchDb.insert('bench', key, value);
            }
        },
        fn: function () {
            batchDb.getBatch('bench', testKeys);
        }
    })
    .on('cycle', function (event) {
        console.log(String(event.target));
        const timeMs = event.target.stats.mean * 1000;
        console.log(`  → ${timeMs.toFixed(2)}ms per operation\n`);
    })
    .on('complete', function () {
        console.log('\n=== Performance Comparison ===');

        const standardScan = this.filter(name => name === 'Standard SCAN 10k')[0];
        const zeroCopyScan = this.filter(name => name === 'Zero-Copy SCAN 10k')[0];
        const standardGet = this.filter(name => name === 'Standard GET 10k')[0];
        const batchGet = this.filter(name => name === 'Batch GET 10k')[0];

        if (standardScan && zeroCopyScan) {
            const scanImprovement = ((standardScan.stats.mean - zeroCopyScan.stats.mean) / standardScan.stats.mean * 100).toFixed(1);
            console.log(`SCAN: ${scanImprovement}% faster with zero-copy`);
        }

        if (standardGet && batchGet) {
            const getImprovement = ((standardGet.stats.mean - batchGet.stats.mean) / standardGet.stats.mean * 100).toFixed(1);
            console.log(`GET: ${getImprovement}% faster with batch API`);
        }
    })
    .run({ 'async': false });
