#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    const char *path = getenv("DYLD_LIBRARY_PATH");
    if (path != NULL) {
        puts(path);
    }
    fputs("uses-dep fixture", stdout);
    for (int i = 1; i < argc; i++) {
        printf(" %s", argv[i]);
    }
    putchar('\n');
    return 0;
}
