/*
  Allocator-hook trace. Installs logging cJSON_Hooks through the public API,
  runs a fixed set of calls, and prints every malloc/free the library makes
  ("M <size>" / "F <size of the freed block>") plus each call's result.

  Build it twice, against cJSON.c and against target/release/libcjson_ffi.a,
  and diff the output (`make hook-trace`). The differences are the
  hook-visible behavior changes listed in MIGRATION.md; parse/print output is
  covered by the parity fixtures, not by this tool.

  Every allocation gets one spare byte for the same reason as in
  tools/cjson-oracle.c: cJSON_strdup (cJSON.c:203-209) copies one byte more
  than it allocates.
*/
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cJSON.h"

#define MAX_LIVE 4096

static void *live_ptr[MAX_LIVE];
static size_t live_size[MAX_LIVE];
static long fail_after = -1;

static void *trace_malloc(size_t size)
{
    void *p;
    size_t i;
    if (fail_after == 0)
    {
        printf("M %lu FAIL\n", (unsigned long)size);
        return NULL;
    }
    if (fail_after > 0)
    {
        fail_after--;
    }
    p = malloc(size + 1);
    if (p == NULL)
    {
        return NULL;
    }
    for (i = 0; i < MAX_LIVE; i++)
    {
        if (live_ptr[i] == NULL)
        {
            live_ptr[i] = p;
            live_size[i] = size;
            break;
        }
    }
    printf("M %lu\n", (unsigned long)size);
    return p;
}

static void trace_free(void *p)
{
    size_t i;
    if (p == NULL)
    {
        return;
    }
    for (i = 0; i < MAX_LIVE; i++)
    {
        if (live_ptr[i] == p)
        {
            printf("F %lu\n", (unsigned long)live_size[i]);
            live_ptr[i] = NULL;
            break;
        }
    }
    if (i == MAX_LIVE)
    {
        printf("F ?\n");
    }
    free(p);
}

static void section(const char *name)
{
    printf("== %s\n", name);
}

static void print_and_free(char *s)
{
    printf("-> %s\n", s == NULL ? "(null)" : s);
    if (s != NULL)
    {
        cJSON_free(s);
    }
}

static const char sample[] = "{\"key\":[\"v\",1,null],\"s\":\"text\"}";

int main(void)
{
    cJSON_Hooks hooks;
    cJSON *root;
    cJSON *copy;
    cJSON *rep;
    long k;

    hooks.malloc_fn = trace_malloc;
    hooks.free_fn = trace_free;
    cJSON_InitHooks(&hooks);

    section("parse");
    root = cJSON_Parse(sample);

    section("print_unformatted");
    print_and_free(cJSON_PrintUnformatted(root));

    section("print_formatted");
    print_and_free(cJSON_Print(root));

    section("print_buffered_4");
    print_and_free(cJSON_PrintBuffered(root, 4, 0));

    section("duplicate");
    copy = cJSON_Duplicate(root, 1);

    section("set_valuestring_longer");
    printf("-> %s\n", cJSON_SetValuestring(cJSON_GetObjectItem(copy, "s"), "a longer text value") == NULL ? "(null)" : "ok");

    section("add_item_to_object");
    printf("-> %d\n", (int)cJSON_AddItemToObject(copy, "added", cJSON_CreateNumber(7)));

    section("replace_item_in_object");
    rep = cJSON_CreateString("new");
    printf("-> %d\n", (int)cJSON_AddItemToObject(root, "tmp", rep));
    printf("-> %d\n", (int)(cJSON_DetachItemViaPointer(root, rep) == rep));
    printf("-> %d\n", (int)cJSON_ReplaceItemInObject(root, "key", rep));

    section("delete");
    cJSON_Delete(root);
    cJSON_Delete(copy);

    for (k = 0; k < 12; k++)
    {
        printf("== parse_fail_after_%ld\n", k);
        fail_after = k;
        root = cJSON_Parse(sample);
        fail_after = -1;
        printf("-> %s\n", root == NULL ? "(null)" : "ok");
        cJSON_Delete(root);
    }

    root = cJSON_Parse(sample);
    for (k = 0; k < 3; k++)
    {
        printf("== print_fail_after_%ld\n", k);
        fail_after = k;
        print_and_free(cJSON_PrintUnformatted(root));
        fail_after = -1;
    }
    cJSON_Delete(root);

    cJSON_InitHooks(NULL);
    return 0;
}
