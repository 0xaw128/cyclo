#include <stdio.h>

#define CHARLIE

void a() {
    int x = 1;
    printf("x: %d\n", x);
    int y = 3;

    if ((x == 2) && (y > 1)) {
        printf("yes\n");
    } else {
        printf("no\n");
    }
}

int b(int x) {
#ifdef BRAVO
    int y = 1;
    printf("y: %d\n", y);
#endif

    printf("x: %d\n", x);

    return 4;
}

void c(int m) {
#ifdef CHARLIE
    int z = 3;
    printf("z: %d\n", z);
#endif
    printf("m: %d\n", m);

    if (m > 2) {
        if (m == 1) {
            printf("yes\n");
        } else {
            if (m == 0) {
                printf("nono\n");
            } else {
                printf("no\n");
            }
            printf("no2\n");
        }
    }
}


int main(int argc, char** argv) {
    int n;

    a();
    n = b(2);
    c(n);

    return 0;
}
