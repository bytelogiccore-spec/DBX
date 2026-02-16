using System;
using System.Text;
using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Running;
using DBX.Dotnet;

namespace DBX.Benchmarks
{
    /// <summary>
    /// FFI Performance Benchmark for DBX .NET Bindings
    /// 
    /// Measures INSERT, GET, and SCAN performance to quantify FFI overhead
    /// compared to Rust Core benchmarks.
    /// 
    /// Run with: dotnet run -c Release --project benchmarks
    /// </summary>
    [MemoryDiagnoser]
    [SimpleJob(warmupCount: 3, iterationCount: 100)]
    public class DbxFfiBenchmarks
    {
        private const int NumEntries = 10_000;
        private (byte[], byte[])[] _testData;

        [GlobalSetup]
        public void Setup()
        {
            _testData = GenerateTestData();
        }

        private static (byte[], byte[])[] GenerateTestData()
        {
            var data = new (byte[], byte[])[NumEntries];
            for (int i = 0; i < NumEntries; i++)
            {
                var key = Encoding.UTF8.GetBytes($"key_{i:D8}");
                var value = Encoding.UTF8.GetBytes($"value_{i:D8}_data");
                data[i] = (key, value);
            }
            return data;
        }

        [Benchmark]
        public void Insert10k()
        {
            using var db = Database.OpenInMemory();
            foreach (var (key, value) in _testData)
            {
                db.Insert("bench", key, value);
            }
        }

        [Benchmark]
        public void Get10k()
        {
            using var db = Database.OpenInMemory();
            
            // Setup: Insert data first
            foreach (var (key, value) in _testData)
            {
                db.Insert("bench", key, value);
            }
            
            // Benchmark: Get all records
            foreach (var (key, _) in _testData)
            {
                db.Get("bench", key);
            }
        }

        [Benchmark]
        public void Scan10k()
        {
            using var db = Database.OpenInMemory();
            
            // Setup: Insert data first
            foreach (var (key, value) in _testData)
            {
                db.Insert("bench", key, value);
            }
            
            // Benchmark: Scan all records
            db.Scan("bench");
        }
    }

    public class Program
    {
        public static void Main(string[] args)
        {
            var summary = BenchmarkRunner.Run<DbxFfiBenchmarks>();
        }
    }
}
