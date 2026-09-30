/*
  Parity oracle driver for the cJSON C-to-Rust port.

  Prints the report specified in tools/DRIVER_FORMAT.md for one fixture file.
  Uses only the public cJSON.h API, so the same source links against the
  original cJSON.c (build/oracle) or the Rust shim libcjson_ffi.a
  (build/oracle-ffi). Must stay byte-for-byte equivalent to
  tools/rust-driver/src/main.rs.

  Usage: cjson-oracle <fixture-path>
  Exit status: 0 after a full report, 2 on usage/read/write errors.

  Allocator hooks: cJSON_strdup in this tree (cJSON.c:203-209, commit 427291d)
  allocates strlen + 1 bytes but copies strlen + 2, so every cJSON_Duplicate
  of a string reads one byte past its source and writes one byte past its
  allocation. That is undefined behavior: depending on heap layout the oracle
  crashes (SIGSEGV/SIGKILL) or runs normally. main() therefore installs hooks
  through the public cJSON_InitHooks API that give every allocation one spare
  byte. The overrun then stays inside the allocation and never reaches
  printed output, so the report shows what cJSON.c computes, deterministically.
  With non-default hooks cJSON uses allocate + memcpy + free instead of
  realloc (cJSON.c cJSON_InitHooks); that path produces the same bytes.
*/

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cJSON.h"

#define READ_CHUNK 65536
#define COMPARE_COST_LIMIT 1000000UL

static int write_failed = 0;

/* One spare byte per allocation; see the allocator hooks note above. */
static void *CJSON_CDECL slack_malloc(size_t size)
{
    if (size == (size_t)-1)
    {
        return NULL;
    }
    return malloc(size + 1);
}

static void CJSON_CDECL slack_free(void *pointer)
{
    free(pointer);
}

static void put_bytes(const char *bytes, size_t length)
{
    if ((length > 0) && (fwrite(bytes, 1, length, stdout) != length))
    {
        write_failed = 1;
    }
}

static void put_str(const char *text)
{
    put_bytes(text, strlen(text));
}

static void put_ulong(unsigned long value)
{
    char digits[32];
    size_t pos = sizeof(digits);
    do
    {
        pos--;
        digits[pos] = (char)('0' + (int)(value % 10UL));
        value /= 10UL;
    } while ((value > 0UL) && (pos > 0));
    put_bytes(digits + pos, sizeof(digits) - pos);
}

static void put_int(int value)
{
    if (value < 0)
    {
        put_str("-");
        put_ulong((unsigned long)(-(long)value));
    }
    else
    {
        put_ulong((unsigned long)value);
    }
}

static unsigned long offset_of(const char *pointer, const char *base)
{
    return (unsigned long)(pointer - base);
}

/* "<tag> <L>\n<bytes>\n" or "<tag> null\n" */
static void put_blob(const char *tag, const char *text)
{
    put_str(tag);
    if (text == NULL)
    {
        put_str(" null\n");
        return;
    }
    put_str(" ");
    put_ulong((unsigned long)strlen(text));
    put_str("\n");
    put_str(text);
    put_str("\n");
}

/* "same" if both NULL or both equal, "null" if candidate is NULL, else "diff" */
static void put_same(const char *tag, const char *candidate, const char *reference)
{
    put_str(tag);
    if (candidate == NULL)
    {
        put_str(" null\n");
    }
    else if ((reference != NULL) && (strcmp(candidate, reference) == 0))
    {
        put_str(" same\n");
    }
    else
    {
        put_str(" diff\n");
    }
}

static int prealloc_one(cJSON *root, size_t length, cJSON_bool format, const char *reference)
{
    char *buf2 = NULL;
    cJSON_bool ok = 0;

    buf2 = (char *)malloc(length + 16);
    if (buf2 == NULL)
    {
        return -1;
    }
    memset(buf2, 0x5A, length + 15);
    buf2[length + 15] = '\0';

    ok = cJSON_PrintPreallocated(root, buf2, (int)length, format);

    put_str(" ");
    put_ulong((unsigned long)length);
    if (ok)
    {
        put_str(":1");
        if (strcmp(buf2, reference) != 0)
        {
            put_str("!");
        }
    }
    else
    {
        put_str(":0");
    }
    free(buf2);
    return 0;
}

static int put_prealloc(const char *tag, cJSON *root, const char *reference, cJSON_bool format)
{
    size_t length = strlen(reference);
    put_str(tag);
    if ((length > 0) && (prealloc_one(root, length - 1, format, reference) != 0))
    {
        return -1;
    }
    if ((prealloc_one(root, length, format, reference) != 0)
        || (prealloc_one(root, length + 1, format, reference) != 0)
        || (prealloc_one(root, length + 5, format, reference) != 0))
    {
        return -1;
    }
    put_str("\n");
    return 0;
}

/*
  Upper bound on the cJSON_Compare calls needed to compare item with an equal
  tree: cJSON_Compare walks an object's members twice and recurses in both
  walks, so the cost doubles per object level. Saturates at
  COMPARE_COST_LIMIT + 1.
*/
static unsigned long compare_cost(const cJSON *item)
{
    unsigned long total = 0;
    const cJSON *child = NULL;
    for (child = item->child; child != NULL; child = child->next)
    {
        total += compare_cost(child);
        if (total > COMPARE_COST_LIMIT)
        {
            return COMPARE_COST_LIMIT + 1;
        }
    }
    if ((item->type & 0xFF) == cJSON_Object)
    {
        total *= 2;
    }
    total += 1;
    return (total > COMPARE_COST_LIMIT) ? (COMPARE_COST_LIMIT + 1) : total;
}

static char *read_fixture(const char *path, size_t *out_length)
{
    FILE *file = NULL;
    char *content = NULL;
    size_t length = 0;
    size_t capacity = READ_CHUNK;
    size_t got = 0;

    file = fopen(path, "rb");
    if (file == NULL)
    {
        return NULL;
    }
    content = (char *)malloc(capacity + 1);
    if (content == NULL)
    {
        fclose(file);
        return NULL;
    }
    for (;;)
    {
        if (length == capacity)
        {
            char *grown = NULL;
            capacity *= 2;
            grown = (char *)realloc(content, capacity + 1);
            if (grown == NULL)
            {
                free(content);
                fclose(file);
                return NULL;
            }
            content = grown;
        }
        got = fread(content + length, 1, capacity - length, file);
        length += got;
        if (got == 0)
        {
            break;
        }
    }
    if (ferror(file))
    {
        free(content);
        fclose(file);
        return NULL;
    }
    fclose(file);
    content[length] = '\0';
    *out_length = length;
    return content;
}

static int report_parse_with_opts(const char *buf)
{
    const char *end = NULL;
    cJSON *root = NULL;
    cJSON *dup = NULL;
    char *compact = NULL;
    char *pretty = NULL;
    char *buffered = NULL;
    char *dup_compact = NULL;
    int status = 0;

    root = cJSON_ParseWithOpts(buf, &end, 0);
    if (root == NULL)
    {
        const char *error = cJSON_GetErrorPtr();
        put_str("opts0 fail pos ");
        put_ulong(offset_of(error, buf));
        put_str("\n");
        if (end != error)
        {
            put_str("opts0 endmismatch\n");
        }
        return 0;
    }

    put_str("opts0 ok end ");
    put_ulong(offset_of(end, buf));
    put_str("\n");

    compact = cJSON_PrintUnformatted(root);
    put_blob("compact", compact);
    pretty = cJSON_Print(root);
    put_blob("pretty", pretty);

    buffered = cJSON_PrintBuffered(root, 0, 1);
    put_same("buffered0", buffered, pretty);
    cJSON_free(buffered);
    buffered = cJSON_PrintBuffered(root, 1, 0);
    put_same("buffered1", buffered, compact);
    cJSON_free(buffered);

    if ((compact != NULL) && (put_prealloc("prealloc_compact", root, compact, 0) != 0))
    {
        status = -1;
    }
    if ((status == 0) && (pretty != NULL) && (put_prealloc("prealloc_pretty", root, pretty, 1) != 0))
    {
        status = -1;
    }

    if (status == 0)
    {
        dup = cJSON_Duplicate(root, 1);
        if (dup == NULL)
        {
            put_str("dup null\n");
            put_str("cmp null\n");
        }
        else
        {
            dup_compact = cJSON_PrintUnformatted(dup);
            put_str("dup ");
            if ((dup_compact == NULL) ? (compact == NULL)
                : ((compact != NULL) && (strcmp(dup_compact, compact) == 0)))
            {
                put_str("same\n");
            }
            else
            {
                put_str("diff\n");
            }
            cJSON_free(dup_compact);
            if (compare_cost(root) > COMPARE_COST_LIMIT)
            {
                put_str("cmp skipped\n");
            }
            else
            {
                put_str("cmp ");
                put_int(cJSON_Compare(root, dup, 1) ? 1 : 0);
                put_str(" ");
                put_int(cJSON_Compare(root, dup, 0) ? 1 : 0);
                put_str("\n");
            }
        }

        put_str("size ");
        put_int(cJSON_GetArraySize(root));
        put_str("\ntype ");
        put_int(root->type);
        put_str("\n");
    }

    cJSON_Delete(dup);
    cJSON_free(compact);
    cJSON_free(pretty);
    cJSON_Delete(root);
    return status;
}

static void report_parse_strict(const char *buf)
{
    const char *end = NULL;
    cJSON *root = cJSON_ParseWithOpts(buf, &end, 1);
    if (root == NULL)
    {
        put_str("opts1 fail pos ");
        put_ulong(offset_of(cJSON_GetErrorPtr(), buf));
    }
    else
    {
        put_str("opts1 ok end ");
        put_ulong(offset_of(end, buf));
    }
    put_str("\n");
    cJSON_Delete(root);
}

static void report_parse_with_length(const char *buf, size_t length)
{
    const char *end = NULL;
    char *compact = NULL;
    cJSON *root = cJSON_ParseWithLengthOpts(buf, length, &end, 0);
    if (root == NULL)
    {
        put_str("len fail pos ");
        put_ulong(offset_of(cJSON_GetErrorPtr(), buf));
        put_str("\n");
        return;
    }
    put_str("len ok end ");
    put_ulong(offset_of(end, buf));
    put_str("\n");
    compact = cJSON_PrintUnformatted(root);
    put_blob("len_compact", compact);
    cJSON_free(compact);
    cJSON_Delete(root);
}

static int report_minify(const char *buf, size_t length)
{
    char *copy = (char *)malloc(length + 1);
    if (copy == NULL)
    {
        return -1;
    }
    memcpy(copy, buf, length + 1);
    cJSON_Minify(copy);
    put_blob("minify", copy);
    free(copy);
    return 0;
}

int main(int argc, char **argv)
{
    char *buf = NULL;
    size_t length = 0;
    int status = 0;
    cJSON_Hooks hooks;

    hooks.malloc_fn = slack_malloc;
    hooks.free_fn = slack_free;
    cJSON_InitHooks(&hooks);

    if (argc != 2)
    {
        fprintf(stderr, "usage: cjson-oracle <fixture-path>\n");
        return 2;
    }
    buf = read_fixture(argv[1], &length);
    if (buf == NULL)
    {
        fprintf(stderr, "cjson-oracle: cannot read %s\n", argv[1]);
        return 2;
    }

    put_str("input_len ");
    put_ulong((unsigned long)length);
    put_str("\n");

    if (report_parse_with_opts(buf) != 0)
    {
        status = 2;
    }
    if (status == 0)
    {
        report_parse_strict(buf);
        report_parse_with_length(buf, length);
        if (report_minify(buf, length) != 0)
        {
            status = 2;
        }
    }
    free(buf);

    if ((fflush(stdout) != 0) || write_failed)
    {
        fprintf(stderr, "cjson-oracle: write to stdout failed\n");
        return 2;
    }
    if (status != 0)
    {
        fprintf(stderr, "cjson-oracle: out of memory\n");
    }
    return status;
}
