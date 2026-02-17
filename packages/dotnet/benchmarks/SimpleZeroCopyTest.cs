using System;
using System.Text;
using System.Diagnostics;
using DBX.Dotnet;

namespace DBX.Benchmarks
{
    /// <summary>
    /// Simple Zero-Copy SCAN Performance Test
    /// </summary>
    public class SimpleZeroCopyTest
    {
        private const int NumEntries = 10_000;

        public static void Main(string[] args)
        {
            Console.WriteLine("============================================================");
            Console.WriteLine("DBX C# Zero-Copy SCAN Test");
            Console.WriteLine("============================================================");

            using var db = Database.OpenInMemory();

            // Insert test data
            Console.WriteLine($"Inserting {NumEntries:N0} entries...");
            for (int i = 0; i < NumEntries; i++)
            {
                var key = Encoding.UTF8.GetBytes($"key_{i:D8}");
                var value = Encoding.UTF8.GetBytes($"value_{i:D8}_data");
                db.Insert("bench", key, value);
            }

            // Warmup
            for (int i = 0; i < 5; i++)
            {
                var _ = db.Scan("bench");
                using var __ = db.ScanZeroCopy("bench");
            }

            // Test Standard SCAN
            var sw = Stopwatch.StartNew();
            for (int i = 0; i < 100; i++)
            {
                var result = db.Scan("bench");
            }
            sw.Stop();
            var standardTime = sw.Elapsed.TotalMilliseconds / 100.0;

            // Test Zero-Copy SCAN
            sw.Restart();
            for (int i = 0; i < 100; i++)
            {
                using var result = db.ScanZeroCopy("bench");
                var count = result.Count;
            }
            sw.Stop();
            var zeroCopyTime = sw.Elapsed.TotalMilliseconds / 100.0;

            // Test Zero-Copy SCAN with parsing
            sw.Restart();
            for (int i = 0; i < 100; i++)
            {
                using var result = db.ScanZeroCopy("bench");
                var pairs = result.ToPairs();
            }
            sw.Stop();
            var zeroCopyParseTime = sw.Elapsed.TotalMilliseconds / 100.0;

            // Results
            Console.WriteLine();
            Console.WriteLine("Results:");
            Console.WriteLine($"Standard SCAN:              {standardTime:F2}ms");
            Console.WriteLine($"Zero-Copy SCAN (no parse):  {zeroCopyTime:F2}ms");
            Console.WriteLine($"Zero-Copy SCAN (w/ parse):  {zeroCopyParseTime:F2}ms");
            Console.WriteLine();

            if (zeroCopyTime < standardTime)
            {
                var speedup = standardTime / zeroCopyTime;
                var improvement = ((standardTime - zeroCopyTime) / standardTime) * 100.0;
                Console.WriteLine($"Speedup (no parse):         {speedup:F2}x");
                Console.WriteLine($"Improvement (no parse):     {improvement:F1}%");
            }

            if (zeroCopyParseTime < standardTime)
            {
                var speedup = standardTime / zeroCopyParseTime;
                var improvement = ((standardTime - zeroCopyParseTime) / standardTime) * 100.0;
                Console.WriteLine($"Speedup (w/ parse):         {speedup:F2}x");
                Console.WriteLine($"Improvement (w/ parse):     {improvement:F1}%");
            }

            Console.WriteLine();
            Console.WriteLine("============================================================");
            Console.WriteLine("Test completed! ✅");
            Console.WriteLine("============================================================");
        }
    }
}
