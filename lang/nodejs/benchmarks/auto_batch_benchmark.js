/**
 * Auto-Batching Performance Benchmark
 * 
 * Compares standard individual get() calls vs auto-batched get() calls
 * 
 * Run with: node benchmarks/auto_batch_benchmark.js
 */

const Benchmark = require('benchmark');
const { Database } = require('../');
const { SmartDatabase } = require('../smart_database');

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
let standardDb, smartDb, testData, testKeys;

const suite = new Benchmark.Suite();

suite
    .add('Standard GET 10k (individual calls)', {
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
    .add('Auto-Batched GET 10k (SmartDatabase)', {
        onStart: function () {
            const rawDb = Database.openInMemory();
            smartDb = new SmartDatabase(rawDb);
            testData = generateTestData();
            testKeys = testData.map(([key]) => key);
            for (const [key, value] of testData) {
                smartDb.insert('bench', key, value);
            }
        },
        fn: async function (deferred) {
            // All get() calls are automatically batched
            await Promise.all(testKeys.map(key => smartDb.get('bench', key)));
            deferred.resolve();
        },
        defer: true
    })
    .add('Manual Batch GET 10k', {
        onStart: function () {
            standardDb = Database.openInMemory();
            testData = generateTestData();
            testKeys = testData.map(([key]) => key);
            for (const [key, value] of testData) {
                standardDb.insert('bench', key, value);
            }
        },
        fn: function () {
            standardDb.getBatch('bench', testKeys);
        }
    })
    .on('cycle', function (event) {
        console.log(String(event.target));
        const timeMs = event.target.stats.mean * 1000;
        console.log(`  → ${timeMs.toFixed(2)}ms per operation\n`);
    })
    .on('complete', function () {
        console.log('\n=== Performance Comparison ===');

        const standard = this.filter(name => name === 'Standard GET 10k (individual calls)')[0];
        const autoBatch = this.filter(name => name === 'Auto-Batched GET 10k (SmartDatabase)')[0];
        const manualBatch = this.filter(name => name === 'Manual Batch GET 10k')[0];

        if (standard && autoBatch) {
            const improvement = ((standard.stats.mean - autoBatch.stats.mean) / standard.stats.mean * 100).toFixed(1);
            console.log(`Auto-batching: ${improvement}% faster than individual calls`);
        }

        if (autoBatch && manualBatch) {
            const overhead = ((autoBatch.stats.mean - manualBatch.stats.mean) / manualBatch.stats.mean * 100).toFixed(1);
            console.log(`Auto-batching overhead: ${overhead}% vs manual batch`);
        }
    })
    .run({ 'async': true });
