# Makefile for DBX C Benchmark

CC = gcc
CFLAGS = -Wall -Wextra -O3 -I../include
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

$(TARGET): benchmark_crud.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(LDLIBS)

run: $(TARGET)
	./$(TARGET)

clean:
	rm -f $(TARGET)

.PHONY: all run clean
