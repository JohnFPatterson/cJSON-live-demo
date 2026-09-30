CJSON_OBJ = cJSON.o
UTILS_OBJ = cJSON_Utils.o
CJSON_LIBNAME = libcjson
UTILS_LIBNAME = libcjson_utils
CJSON_TEST = cJSON_test

CJSON_TEST_SRC = cJSON.c test.c

LDLIBS = -lm

LIBVERSION = 1.7.19
CJSON_SOVERSION = 1
UTILS_SOVERSION = 1

CJSON_SO_LDFLAG=-Wl,-soname=$(CJSON_LIBNAME).so.$(CJSON_SOVERSION)
UTILS_SO_LDFLAG=-Wl,-soname=$(UTILS_LIBNAME).so.$(UTILS_SOVERSION)

PREFIX ?= /usr/local
INCLUDE_PATH ?= include/cjson
LIBRARY_PATH ?= lib

INSTALL_INCLUDE_PATH = $(DESTDIR)$(PREFIX)/$(INCLUDE_PATH)
INSTALL_LIBRARY_PATH = $(DESTDIR)$(PREFIX)/$(LIBRARY_PATH)

INSTALL ?= cp -a

CC = gcc -std=c89

# validate gcc version for use fstack-protector-strong
MIN_GCC_VERSION = "4.9"
GCC_VERSION := "`$(CC) -dumpversion`"
IS_GCC_ABOVE_MIN_VERSION := $(shell expr "$(GCC_VERSION)" ">=" "$(MIN_GCC_VERSION)")
ifeq "$(IS_GCC_ABOVE_MIN_VERSION)" "1"
    CFLAGS += -fstack-protector-strong
else
    CFLAGS += -fstack-protector
endif

PIC_FLAGS = -fPIC
R_CFLAGS = $(PIC_FLAGS) -pedantic -Wall -Werror -Wstrict-prototypes -Wwrite-strings -Wshadow -Winit-self -Wcast-align -Wformat=2 -Wmissing-prototypes -Wstrict-overflow=2 -Wcast-qual -Wc++-compat -Wundef -Wswitch-default -Wconversion $(CFLAGS)

uname := $(shell sh -c 'uname -s 2>/dev/null || echo false')

#library file extensions
SHARED = so
STATIC = a

## create dynamic (shared) library on Darwin (base OS for MacOSX and IOS)
ifeq (Darwin, $(uname))
	SHARED = dylib
	CJSON_SO_LDFLAG = ""
	UTILS_SO_LDFLAG = ""
endif

#cJSON library names
CJSON_SHARED = $(CJSON_LIBNAME).$(SHARED)
CJSON_SHARED_VERSION = $(CJSON_LIBNAME).$(SHARED).$(LIBVERSION)
CJSON_SHARED_SO = $(CJSON_LIBNAME).$(SHARED).$(CJSON_SOVERSION)
CJSON_STATIC = $(CJSON_LIBNAME).$(STATIC)

#cJSON_Utils library names
UTILS_SHARED = $(UTILS_LIBNAME).$(SHARED)
UTILS_SHARED_VERSION = $(UTILS_LIBNAME).$(SHARED).$(LIBVERSION)
UTILS_SHARED_SO = $(UTILS_LIBNAME).$(SHARED).$(UTILS_SOVERSION)
UTILS_STATIC = $(UTILS_LIBNAME).$(STATIC)

SHARED_CMD = $(CC) -shared -o

.PHONY: all shared static tests clean install

all: shared static tests

shared: $(CJSON_SHARED) $(UTILS_SHARED)

static: $(CJSON_STATIC) $(UTILS_STATIC)

tests: $(CJSON_TEST)

test: tests
	./$(CJSON_TEST)

.c.o:
	$(CC) -c $(R_CFLAGS) $<

#tests
#cJSON
$(CJSON_TEST): $(CJSON_TEST_SRC) cJSON.h
	$(CC) $(R_CFLAGS) $(CJSON_TEST_SRC)  -o $@ $(LDLIBS) -I.

#static libraries
#cJSON
$(CJSON_STATIC): $(CJSON_OBJ)
	$(AR) rcs $@ $<
#cJSON_Utils
$(UTILS_STATIC): $(UTILS_OBJ)
	$(AR) rcs $@ $<

#shared libraries .so.1.0.0
#cJSON
$(CJSON_SHARED_VERSION): $(CJSON_OBJ)
	$(CC) -shared -o $@ $< $(CJSON_SO_LDFLAG) $(LDFLAGS)
#cJSON_Utils
$(UTILS_SHARED_VERSION): $(UTILS_OBJ)
	$(CC) -shared -o $@ $< $(CJSON_OBJ) $(UTILS_SO_LDFLAG) $(LDFLAGS)

#objects
#cJSON
$(CJSON_OBJ): cJSON.c cJSON.h
#cJSON_Utils
$(UTILS_OBJ): cJSON_Utils.c cJSON_Utils.h cJSON.h


#links .so -> .so.1 -> .so.1.0.0
#cJSON
$(CJSON_SHARED_SO): $(CJSON_SHARED_VERSION)
	ln -s $(CJSON_SHARED_VERSION) $(CJSON_SHARED_SO)
$(CJSON_SHARED): $(CJSON_SHARED_SO)
	ln -s $(CJSON_SHARED_SO) $(CJSON_SHARED)
#cJSON_Utils
$(UTILS_SHARED_SO): $(UTILS_SHARED_VERSION)
	ln -s $(UTILS_SHARED_VERSION) $(UTILS_SHARED_SO)
$(UTILS_SHARED): $(UTILS_SHARED_SO)
	ln -s $(UTILS_SHARED_SO) $(UTILS_SHARED)

#install
#cJSON
install-cjson:
	mkdir -p $(INSTALL_LIBRARY_PATH) $(INSTALL_INCLUDE_PATH)
	$(INSTALL) cJSON.h $(INSTALL_INCLUDE_PATH)
	$(INSTALL) $(CJSON_SHARED) $(CJSON_SHARED_SO) $(CJSON_SHARED_VERSION) $(INSTALL_LIBRARY_PATH)
#cJSON_Utils
install-utils: install-cjson
	$(INSTALL) cJSON_Utils.h $(INSTALL_INCLUDE_PATH)
	$(INSTALL) $(UTILS_SHARED) $(UTILS_SHARED_SO) $(UTILS_SHARED_VERSION) $(INSTALL_LIBRARY_PATH)

install: install-cjson install-utils

#uninstall
#cJSON
uninstall-cjson: uninstall-utils
	$(RM) $(INSTALL_LIBRARY_PATH)/$(CJSON_SHARED)
	$(RM) $(INSTALL_LIBRARY_PATH)/$(CJSON_SHARED_VERSION)
	$(RM) $(INSTALL_LIBRARY_PATH)/$(CJSON_SHARED_SO)
	$(RM) $(INSTALL_INCLUDE_PATH)/cJSON.h
	
#cJSON_Utils
uninstall-utils:
	$(RM) $(INSTALL_LIBRARY_PATH)/$(UTILS_SHARED)
	$(RM) $(INSTALL_LIBRARY_PATH)/$(UTILS_SHARED_VERSION)
	$(RM) $(INSTALL_LIBRARY_PATH)/$(UTILS_SHARED_SO)
	$(RM) $(INSTALL_INCLUDE_PATH)/cJSON_Utils.h

remove-dir:
	$(if $(wildcard $(INSTALL_LIBRARY_PATH)/*.*),,rmdir $(INSTALL_LIBRARY_PATH))
	$(if $(wildcard $(INSTALL_INCLUDE_PATH)/*.*),,rmdir $(INSTALL_INCLUDE_PATH))

uninstall: uninstall-utils uninstall-cjson remove-dir

clean:
	$(RM) $(CJSON_OBJ) $(UTILS_OBJ) #delete object files
	$(RM) $(CJSON_SHARED) $(CJSON_SHARED_VERSION) $(CJSON_SHARED_SO) $(CJSON_STATIC) #delete cJSON
	$(RM) $(UTILS_SHARED) $(UTILS_SHARED_VERSION) $(UTILS_SHARED_SO) $(UTILS_STATIC) #delete cJSON_Utils
	$(RM) $(CJSON_TEST)  #delete test

# ---- Rust port / parity ----
# Targets for the C-to-Rust port (cjson-core / cjson-ffi). Everything above this
# line is the original cJSON Makefile and is unchanged. Build outputs go to
# build/ (gitignored) and target/ (cargo). See tools/DRIVER_FORMAT.md and
# parity.sh.
.PHONY: oracle rust-driver ffi-lib oracle-ffi c-unity ffi-unity legacy-diff hook-trace parity

PARITY_BUILD = build
# Pinned so the binaries land where .cursor/parity.json and parity.sh look for
# them, even when the environment sets CARGO_TARGET_DIR elsewhere.
CARGO_TARGET = target
FFI_LIB = $(CARGO_TARGET)/release/libcjson_ffi.a
# System libraries a Rust staticlib needs when linked into a C program.
ifeq (Darwin, $(uname))
FFI_LDLIBS = -lm
else
FFI_LDLIBS = -lm -lpthread -ldl
endif
# The non-Utils suites listed in tests/CMakeLists.txt (unity_tests), same order.
C_UNITY_SUITES = parse_examples parse_number parse_hex4 parse_string parse_array \
	parse_object parse_value print_string print_number print_array print_object \
	print_value misc_tests parse_with_opts compare_tests cjson_add \
	readme_examples minify_tests

oracle: $(PARITY_BUILD)/oracle

# Oracle binaries are linked to a temporary name and renamed into place: on
# macOS, overwriting a binary that another process is running gets that
# process killed (SIGKILL), and parity runs may overlap with rebuilds.
$(PARITY_BUILD)/oracle: tools/cjson-oracle.c cJSON.c cJSON.h
	@mkdir -p $(PARITY_BUILD)
	$(CC) $(R_CFLAGS) -I. tools/cjson-oracle.c cJSON.c -o $@.tmp $(LDLIBS)
	mv -f $@.tmp $@

rust-driver:
	cargo build --release --target-dir $(CARGO_TARGET) -p rust-driver

ffi-lib:
	cargo build --release --target-dir $(CARGO_TARGET) -p cjson-ffi

oracle-ffi: ffi-lib
	@mkdir -p $(PARITY_BUILD)
	$(CC) $(R_CFLAGS) -I. tools/cjson-oracle.c $(FFI_LIB) -o $(PARITY_BUILD)/oracle-ffi.tmp $(FFI_LDLIBS)
	mv -f $(PARITY_BUILD)/oracle-ffi.tmp $(PARITY_BUILD)/oracle-ffi

# Original Unity suites against the original C. Each tests/X.c includes
# ../cJSON.c through tests/common.h. Unity itself is built without -Werror, as
# tests/CMakeLists.txt does. Suites run from $(PARITY_BUILD)/c-unity so that
# relative paths such as inputs/test1 resolve (tests/parse_examples.c:61).
c-unity:
	@mkdir -p $(PARITY_BUILD)/c-unity/inputs
	cp tests/inputs/* $(PARITY_BUILD)/c-unity/inputs/
	$(CC) -c tests/unity/src/unity.c -o $(PARITY_BUILD)/c-unity/unity.o
	@set -e; for t in $(C_UNITY_SUITES); do \
		echo "$(CC) $(R_CFLAGS) -Itests/unity/src tests/$$t.c $(PARITY_BUILD)/c-unity/unity.o -o $(PARITY_BUILD)/c-unity/$$t $(LDLIBS)"; \
		$(CC) $(R_CFLAGS) -Itests/unity/src tests/$$t.c $(PARITY_BUILD)/c-unity/unity.o -o $(PARITY_BUILD)/c-unity/$$t $(LDLIBS); \
	done
	@failed=""; for t in $(C_UNITY_SUITES); do \
		echo "== c-unity: $$t"; \
		if ! (cd $(PARITY_BUILD)/c-unity && ./$$t > $$t.log 2>&1); then failed="$$failed $$t"; fi; \
		tail -n 3 $(PARITY_BUILD)/c-unity/$$t.log; \
	done; \
	if [ -n "$$failed" ]; then echo "c-unity FAILED:$$failed"; exit 1; fi; \
	echo "c-unity: all $(words $(C_UNITY_SUITES)) suites passed"

# Public-API Unity suites against the Rust library (see tools/ffi-unity/run.sh
# for the cases it removes and why).
ffi-unity: ffi-lib
	CC="$(CC)" CFLAGS="$(R_CFLAGS)" FFI_LIB="$(FFI_LIB)" FFI_LDLIBS="$(FFI_LDLIBS)" sh tools/ffi-unity/run.sh

# The original test.c driver linked against cJSON.c and against the Rust
# library; stdout must be identical.
legacy-diff: ffi-lib
	@mkdir -p $(PARITY_BUILD)/legacy
	$(CC) $(R_CFLAGS) -I. test.c cJSON.c -o $(PARITY_BUILD)/legacy/test-c $(LDLIBS)
	$(CC) $(R_CFLAGS) -I. test.c $(FFI_LIB) -o $(PARITY_BUILD)/legacy/test-ffi $(FFI_LDLIBS)
	$(PARITY_BUILD)/legacy/test-c > $(PARITY_BUILD)/legacy/test-c.out
	$(PARITY_BUILD)/legacy/test-ffi > $(PARITY_BUILD)/legacy/test-ffi.out
	diff $(PARITY_BUILD)/legacy/test-c.out $(PARITY_BUILD)/legacy/test-ffi.out
	@echo "legacy-diff: test.c output identical ($$(wc -c < $(PARITY_BUILD)/legacy/test-c.out | tr -d ' ') bytes)"

# Allocator-hook call sequences, cJSON.c vs the Rust library. Exits nonzero
# while the differences recorded in MIGRATION.md (CH-1 to CH-3) remain.
hook-trace: ffi-lib
	@mkdir -p $(PARITY_BUILD)/hook-trace
	$(CC) $(R_CFLAGS) -I. tools/hook-trace.c cJSON.c -o $(PARITY_BUILD)/hook-trace/c $(LDLIBS)
	$(CC) $(R_CFLAGS) -I. tools/hook-trace.c $(FFI_LIB) -o $(PARITY_BUILD)/hook-trace/ffi $(FFI_LDLIBS)
	$(PARITY_BUILD)/hook-trace/c > $(PARITY_BUILD)/hook-trace/c.out
	$(PARITY_BUILD)/hook-trace/ffi > $(PARITY_BUILD)/hook-trace/ffi.out
	diff $(PARITY_BUILD)/hook-trace/c.out $(PARITY_BUILD)/hook-trace/ffi.out

parity:
	./parity.sh
