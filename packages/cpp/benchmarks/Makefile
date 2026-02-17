# Makefile for DBX C++ Benchmark

CXX = g++
CXXFLAGS = -std=c++17 -Wall -Wextra -O3 -I../include -I../../c/include
LDFLAGS = -L../../../target/release

# Platform-specific settings
ifeq ($(OS),Windows_NT)
    LDLIBS = -ldbx_ffi -lws2_32 -luserenv -lbcrypt
    EXE = .exe
else
    UNAME_S := $(shell uname -s)
    ifeq ($(UNAME_S),Darwin)
        LDLIBS = -ldbx_ffi -framework Security
    else
        LDLIBS = -ldbx_ffi -lpthread -ldl
    endif
    EXE =
endif

TARGET = benchmark_crud$(EXE)

all: $(TARGET)

$(TARGET): benchmark_crud.cpp
	$(CXX) $(CXXFLAGS) -o $@ $< $(LDFLAGS) $(LDLIBS)

run: $(TARGET)
	./$(TARGET)

clean:
	rm -f $(TARGET)

.PHONY: all run clean
