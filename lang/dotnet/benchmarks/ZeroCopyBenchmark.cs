using System;
using System.Text;
using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Running;
using DBX.Dotnet;

namespace DBX.Benchmarks
{
    /// <summary>
    /// Zero-Copy SCAN Performance Benchmark
    /// 
    /// Compares standard SCAN vs Zero-Copy SCAN performance
    /// 
    /// Run with: dotnet run -c Release --project benchmarks
    /// </summary>
    [MemoryDiagnoser]
    [SimpleJob(warmupCount: 3, iterationCount: 100)]
    public class ZeroCopyBenchmark
    {
        private const int NumEntries = 10_000;
        private Database _db;

        [GlobalSetup]
        public void Setup()
        {
            _db = Database.OpenInMemory();
            
            // Insert test data
            for (int i = 0; i < NumEntries; i++)
            {
                var key = Encoding.UTF8.GetBytes($"key_{i:D8}");
                var value = Encoding.UTF8.GetBytes($"value_{i:D8}_data");
                _db.Insert("bench", key, value);
            }
        }

        [GlobalCleanup]
        public void Cleanup()
        {
            _db?.Dispose();
        }

        [Benchmark(Baseline = true)]
        public void StandardScan()
        {
            var result = _db.Scan("bench");
            // Force enumeration
            foreach (var _ in result) { }
        }

        [Benchmark]
        public void ZeroCopyScan()
        {
            using var result = _db.ScanZeroCopy("bench");
            var count = result.Count;
            // Access raw data
            var data = result.GetRawData();
        }

        [Benchmark]
        public void ZeroCopyScanWithParsing()
        {
            using var result = _db.ScanZeroCopy("bench");
            var pairs = result.ToPairs();
            // Force enumeration
            foreach (var _ in pairs) { }
        }
    }

    public class Program
    {
        public static void Main(string[] args)
        {
            var summary = BenchmarkRunner.Run<ZeroCopyBenchmark>();
        }
    }
}
