# DBX C/C++ Bindings

C and C++ interfaces for the DBX high-performance embedded database.

## C Interface

### Header

Include the C header:
```c
#include "dbx.h"
```

### Example

```c
#include <stdio.h>
#include "dbx.h"

int main() {
    DbxHandle* db = dbx_open("my_database.db");
    
    dbx_insert(db, "users", 
               (uint8_t*)"user:1", 6,
               (uint8_t*)"Alice", 5);
    
    uint8_t* value = NULL;
    size_t value_len = 0;
    dbx_get(db, "users", (uint8_t*)"user:1", 6, &value, &value_len);
    
    printf("%.*s\n", (int)value_len, value);
    dbx_free_value(value, value_len);
    
    dbx_close(db);
    return 0;
}
```

### Building

```bash
cd bindings/c/examples
make
./basic_crud
```

## C++ Interface

### Header

Include the C++ wrapper:
```cpp
#include "dbx.hpp"
```

### Example

```cpp
#include <iostream>
#include "dbx.hpp"

using namespace dbx;

int main() {
    try {
        auto db = Database::openInMemory();
        
        db.insert("users", "user:1", "Alice");
        
        if (auto value = db.getString("users", "user:1")) {
            std::cout << *value << std::endl;
        }
        
    } catch (const DatabaseError& e) {
        std::cerr << "Error: " << e.what() << std::endl;
    }
    
    return 0;
}
```

### Building

```bash
cd bindings/cpp/examples
make
./basic_crud
```

## Features

### C Interface
- ✅ Simple C API
- ✅ Manual memory management
- ✅ Error codes

### C++ Interface
- ✅ RAII (automatic resource management)
- ✅ Modern C++17
- ✅ `std::optional` for nullable returns
- ✅ Move semantics
- ✅ Exception-based error handling

## API Reference

See [`dbx.h`](include/dbx.h) for C API and [`dbx.hpp`](../cpp/include/dbx.hpp) for C++ API.

## License

Dual-licensed under MIT or Apache-2.0.
