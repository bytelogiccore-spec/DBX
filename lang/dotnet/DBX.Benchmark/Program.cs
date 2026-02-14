using System;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Collections.Generic;
using Microsoft.Data.Sqlite;
using DBX.Client;

namespace DBX.Benchmark
{
    class Program
    {
        private const int Iterations = 10000;
        private const string TableName = "benchmark";
        private const string DbxBase = "./dbx_bench_data_";
        private const string SqliteBase = "sqlite_bench_";

        static void Main(string[] args)
        {
            Console.WriteLine("=== DBX vs SQLite Data Integrity & Performance Check ===");
            Console.WriteLine();

            try
            {
                // 1. Data Integrity & Content Check (Disk)
                Console.WriteLine("--- [Step 1] Data Integrity Check (Disk) ---");
                string dbxPath = DbxBase + "integrity";
                string sqlPath = SqliteBase + "integrity.db";
                Cleanup(dbxPath, sqlPath);
                
                VerifyDataIntegrity(dbxPath, sqlPath);

                // 2. Performance Comparison (Lazy WAL vs SQLite Disk/Memory)
                Console.WriteLine("\n--- [Step 2] Performance Comparison ---");
                RunDbxBenchmark(DbxBase + "perf", true, DurabilityLevel.Lazy, false); // DBX Disk
                RunDbxBenchmark(null, true, DurabilityLevel.Lazy, true);              // DBX Memory
                RunSqliteBenchmark(SqliteBase + "perf.db", true);                    // SQLite Disk
                RunSqliteBenchmark(":memory:", true);                                // SQLite Memory

                Console.WriteLine("\nAll Checks Completed.");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"\n[ERROR] Benchmark failed: {ex.Message}");
                Console.WriteLine(ex.StackTrace);
            }
        }

        static void Cleanup(string dbx, string sql)
        {
            try { if (Directory.Exists(dbx)) Directory.Delete(dbx, true); } catch {}
            try { if (sql != null && File.Exists(sql)) File.Delete(sql); } catch {}
        }

        static void VerifyDataIntegrity(string dbxPath, string sqlPath)
        {
            Console.WriteLine("Inserting same data into both DBX and SQLite...");
            
            var testData = new Dictionary<string, string>
            {
                { "user:1", "Alice in Wonderland" },
                { "user:2", "Bob the Builder" },
                { "user:100", "Special Character Test: !@#$%^&*()" },
                { "user:empty", "" }
            };

            // DBX Insert
            using (var db = new DbxDatabase(dbxPath))
            {
                using (var tx = db.BeginTransaction())
                {
                    foreach (var kvp in testData)
                    {
                        tx.Insert(TableName, Encoding.UTF8.GetBytes(kvp.Key), Encoding.UTF8.GetBytes(kvp.Value));
                    }
                    tx.Commit();
                }
            }

            // SQLite Insert
            using (var connection = new SqliteConnection($"Data Source={sqlPath}"))
            {
                connection.Open();
                using (var cmd = connection.CreateCommand())
                {
                    cmd.CommandText = $"CREATE TABLE IF NOT EXISTS {TableName} (id TEXT PRIMARY KEY, val BLOB)";
                    cmd.ExecuteNonQuery();
                }
                
                using (var transaction = connection.BeginTransaction())
                {
                    foreach (var kvp in testData)
                    {
                        using (var cmd = connection.CreateCommand())
                        {
                            cmd.Transaction = transaction;
                            cmd.CommandText = $"INSERT OR REPLACE INTO {TableName} (id, val) VALUES ($id, $val)";
                            cmd.Parameters.AddWithValue("$id", kvp.Key);
                            cmd.Parameters.AddWithValue("$val", Encoding.UTF8.GetBytes(kvp.Value));
                            cmd.ExecuteNonQuery();
                        }
                    }
                    transaction.Commit();
                }
            }

            Console.WriteLine("\nVerifying Data Retrieval...");
            bool allMatch = true;

            using (var db = new DbxDatabase(dbxPath))
            using (var connection = new SqliteConnection($"Data Source={sqlPath}"))
            {
                connection.Open();
                foreach (var kvp in testData)
                {
                    // DBX Get
                    byte[] dbxBytes = db.Get(TableName, Encoding.UTF8.GetBytes(kvp.Key));
                    string dbxVal = dbxBytes != null ? Encoding.UTF8.GetString(dbxBytes) : "[NULL]";

                    // SQLite Get
                    string sqlVal = "[NULL]";
                    using (var cmd = connection.CreateCommand())
                    {
                        cmd.CommandText = $"SELECT val FROM {TableName} WHERE id = $id";
                        cmd.Parameters.AddWithValue("$id", kvp.Key);
                        using (var reader = cmd.ExecuteReader())
                        {
                            if (reader.Read())
                            {
                                byte[] sqlBytes = (byte[])reader[0];
                                sqlVal = Encoding.UTF8.GetString(sqlBytes);
                            }
                        }
                    }

                    bool match = dbxVal == sqlVal && dbxVal == kvp.Value;
                    Console.WriteLine($"Key: {kvp.Key,-10} | Correct: {kvp.Value,-30} | DBX: {dbxVal,-30} | Match: {(match ? "OK" : "FAIL")}");
                    if (!match) allMatch = false;
                }
            }

            if (allMatch)
                Console.WriteLine("\n[SUCCESS] DBX and SQLite data is exactly the same!");
            else
                throw new Exception("Data mismatch detected between DBX and SQLite!");
        }

        static void RunDbxBenchmark(string path, bool useTransaction, DurabilityLevel durability, bool isInMemory)
        {
            var count = Iterations;
            var sw = new Stopwatch();
            using (var db = isInMemory ? DbxDatabase.CreateInMemory() : new DbxDatabase(path))
            {
                db.SetDurability(durability);
                sw.Start();
                using (var tx = db.BeginTransaction())
                {
                    for (int i = 0; i < count; i++)
                    {
                        tx.Insert(TableName, Encoding.UTF8.GetBytes($"key_{i}"), Encoding.UTF8.GetBytes($"value_{i}"));
                    }
                    tx.Commit();
                }
                sw.Stop();
                Console.WriteLine($"[DBX] {(isInMemory ? "Memory" : "Disk")}: Insert {count} records: {sw.ElapsedMilliseconds}ms ({count/sw.Elapsed.TotalSeconds:F0} ops/sec)");

                sw.Restart();
                for (int i = 0; i < count; i++)
                {
                    db.Get(TableName, Encoding.UTF8.GetBytes($"key_{i}"));
                }
                sw.Stop();
                Console.WriteLine($"[DBX] {(isInMemory ? "Memory" : "Disk")}: Get {count} records: {sw.ElapsedMilliseconds}ms ({count/sw.Elapsed.TotalSeconds:F0} ops/sec)");
            }
        }

        static void RunSqliteBenchmark(string path, bool useTransaction)
        {
            var count = Iterations;
            var sw = new Stopwatch();
            using (var connection = new SqliteConnection($"Data Source={path}"))
            {
                connection.Open();
                using (var cmd = connection.CreateCommand())
                {
                    cmd.CommandText = $"CREATE TABLE IF NOT EXISTS {TableName} (id TEXT PRIMARY KEY, val BLOB)";
                    cmd.ExecuteNonQuery();
                }
                sw.Start();
                using (var transaction = connection.BeginTransaction())
                {
                    using (var cmd = connection.CreateCommand())
                    {
                        cmd.Transaction = transaction;
                        cmd.CommandText = $"INSERT OR REPLACE INTO {TableName} (id, val) VALUES ($id, $val)";
                        var idParam = cmd.CreateParameter(); idParam.ParameterName = "$id"; cmd.Parameters.Add(idParam);
                        var valParam = cmd.CreateParameter(); valParam.ParameterName = "$val"; cmd.Parameters.Add(valParam);
                        for (int i = 0; i < count; i++)
                        {
                            idParam.Value = $"key_{i}";
                            valParam.Value = Encoding.UTF8.GetBytes($"value_{i}");
                            cmd.ExecuteNonQuery();
                        }
                    }
                    transaction.Commit();
                }
                sw.Stop();
                Console.WriteLine($"[SQLite] {(path == ":memory:" ? "Memory" : "Disk")}: Insert {count} records: {sw.ElapsedMilliseconds}ms ({count/sw.Elapsed.TotalSeconds:F0} ops/sec)");

                sw.Restart();
                for (int i = 0; i < count; i++)
                {
                    using (var cmd = connection.CreateCommand())
                    {
                        cmd.CommandText = $"SELECT val FROM {TableName} WHERE id = $id";
                        cmd.Parameters.AddWithValue("$id", $"key_{i}");
                        using (var reader = cmd.ExecuteReader()) { if (reader.Read()) { var r = (byte[])reader[0]; } }
                    }
                }
                sw.Stop();
                Console.WriteLine($"[SQLite] {(path == ":memory:" ? "Memory" : "Disk")}: Get {count} records: {sw.ElapsedMilliseconds}ms ({count/sw.Elapsed.TotalSeconds:F0} ops/sec)");
            }
        }
    }
}
