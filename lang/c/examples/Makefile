# Makefile for DBX C Example

CC = gcc
CFLAGS = -Wall -Wextra -I../include
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

TARGET = basic_crud$(EXE)
TX_TARGET = transaction_example$(EXE)

all: $(TARGET) $(TX_TARGET)

$(TARGET): basic_crud.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(LDLIBS)

$(TX_TARGET): transaction_example.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(LDLIBS)

clean:
	rm -f $(TARGET) $(TX_TARGET)

.PHONY: all clean
