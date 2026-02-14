using System;
using System.Diagnostics;
using System.Text;
using DBX.Dotnet;
using Microsoft.Data.Sqlite;

class Program
{
    const int N = 10000;

    static void Main()
    {
        Console.WriteLine("=".PadRight(60, '='));
        Console.WriteLine("DBX Native (CsBindgen) vs SQLite - Performance Comparison");
        Console.WriteLine("=".PadRight(60, '='));
        Console.WriteLine($"\nRunning benchmarks with {N:N0} operations...\n");

        // Benchmark DBX Native
        Console.WriteLine("Benchmarking DBX Native (CsBindgen)...");
        var (dbxInsert, dbxGet, dbxDelete) = BenchmarkDbxNative();
        PrintResults("DBX Native (CsBindgen)", dbxInsert, dbxGet, dbxDelete);

        // Benchmark SQLite
        Console.WriteLine("\nBenchmarking SQLite (In-Memory)...");
        var (sqlInsert, sqlGet, sqlDelete) = BenchmarkSqlite();
        PrintResults("SQLite (In-Memory)", sqlInsert, sqlGet, sqlDelete);

        // Comparison
        Console.WriteLine("\n" + "=".PadRight(60, '='));
        Console.WriteLine("Performance Comparison (DBX vs SQLite):");
        Console.WriteLine("=".PadRight(60, '='));
        Console.WriteLine($"INSERT: DBX is {sqlInsert / dbxInsert:F2}x faster");
        Console.WriteLine($"GET:    DBX is {sqlGet / dbxGet:F2}x faster");
        Console.WriteLine($"DELETE: DBX is {sqlDelete / dbxDelete:F2}x faster");
        
        Console.WriteLine("\n" + "=".PadRight(60, '='));
        Console.WriteLine("Benchmark completed!");
        Console.WriteLine("=".PadRight(60, '='));
    }

    static (double, double, double) BenchmarkDbxNative()
    {
        using var db = Database.OpenInMemory();
        var sw = Stopwatch.StartNew();

        // INSERT with transaction
        sw.Restart();
        using (var tx = db.BeginTransaction())
        {
            for (int i = 0; i < N; i++)
            {
                var key = Encoding.UTF8.GetBytes($"key:{i}");
                var value = Encoding.UTF8.GetBytes($"value:{i}");
                tx.Insert("bench", key, value);
            }
            tx.Commit();
        }
        var insertTime = sw.Elapsed.TotalSeconds;

        // GET
        sw.Restart();
        for (int i = 0; i < N; i++)
        {
            var key = Encoding.UTF8.GetBytes($"key:{i}");
            var _ = db.Get("bench", key);
        }
        var getTime = sw.Elapsed.TotalSeconds;

        // DELETE with transaction
        sw.Restart();
        using (var tx = db.BeginTransaction())
        {
            for (int i = 0; i < N; i++)
            {
                var key = Encoding.UTF8.GetBytes($"key:{i}");
                tx.Delete("bench", key);
            }
            tx.Commit();
        }
        var deleteTime = sw.Elapsed.TotalSeconds;

        return (insertTime, getTime, deleteTime);
    }

    static (double, double, double) BenchmarkSqlite()
    {
        using var connection = new SqliteConnection("Data Source=:memory:");
        connection.Open();

        // Create table
        using (var cmd = connection.CreateCommand())
        {
            cmd.CommandText = "CREATE TABLE bench (key BLOB PRIMARY KEY, value BLOB)";
            cmd.ExecuteNonQuery();
        }

        var sw = Stopwatch.StartNew();

        // INSERT with transaction
        sw.Restart();
        using (var transaction = connection.BeginTransaction())
        {
            for (int i = 0; i < N; i++)
            {
                using var cmd = connection.CreateCommand();
                cmd.CommandText = "INSERT INTO bench (key, value) VALUES (@key, @value)";
                cmd.Parameters.AddWithValue("@key", Encoding.UTF8.GetBytes($"key:{i}"));
                cmd.Parameters.AddWithValue("@value", Encoding.UTF8.GetBytes($"value:{i}"));
                cmd.ExecuteNonQuery();
            }
            transaction.Commit();
        }
        var insertTime = sw.Elapsed.TotalSeconds;

        // GET
        sw.Restart();
        for (int i = 0; i < N; i++)
        {
            using var cmd = connection.CreateCommand();
            cmd.CommandText = "SELECT value FROM bench WHERE key = @key";
            cmd.Parameters.AddWithValue("@key", Encoding.UTF8.GetBytes($"key:{i}"));
            var _ = cmd.ExecuteScalar();
        }
        var getTime = sw.Elapsed.TotalSeconds;

        // DELETE with transaction
        sw.Restart();
        using (var transaction = connection.BeginTransaction())
        {
            for (int i = 0; i < N; i++)
            {
                using var cmd = connection.CreateCommand();
                cmd.CommandText = "DELETE FROM bench WHERE key = @key";
                cmd.Parameters.AddWithValue("@key", Encoding.UTF8.GetBytes($"key:{i}"));
                cmd.ExecuteNonQuery();
            }
            transaction.Commit();
        }
        var deleteTime = sw.Elapsed.TotalSeconds;

        return (insertTime, getTime, deleteTime);
    }

    static void PrintResults(string name, double insertTime, double getTime, double deleteTime)
    {
        Console.WriteLine($"\n{name}:");
        Console.WriteLine($"  INSERT: {insertTime:F4}s ({N / insertTime:N0} ops/sec)");
        Console.WriteLine($"  GET:    {getTime:F4}s ({N / getTime:N0} ops/sec)");
        Console.WriteLine($"  DELETE: {deleteTime:F4}s ({N / deleteTime:N0} ops/sec)");
    }
}
