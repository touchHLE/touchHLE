

// === Declarations ===

// We don't have any system headers for iPhone OS, so we must declare everything
// ourselves rather than #include'ing.


// <stddef.h>
#define NULL ((void *)0)
typedef unsigned long size_t;

// <errno.h>
int *__error(void);
#define errno (*__error())

// <stdarg.h>
typedef __builtin_va_list va_list;
#define va_start(a, b) __builtin_va_start(a, b)
#define va_arg(a, b) __builtin_va_arg(a, b)
#define va_end(a) __builtin_va_end(a)

// <stdio.h>
typedef struct FILE FILE;
FILE *fopen(const char *, const char *);
int fclose(FILE *);
int sscanf(const char *, const char *, ...);
int printf(const char *, ...);
int vsnprintf(char *, size_t, const char *, va_list);

// <stdlib.h>
#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1
void exit(int);
void free(void *);
void *malloc(size_t);
void qsort(void *, size_t, size_t, int (*)(const void *, const void *));
void *realloc(void *, size_t);

// <string.h>
int memcmp(const void *, const void *, size_t);
void *memmove(void *, const void *, size_t);
void *memset(void *str, int c, unsigned long n);
int strcmp(const char *, const char *);

// <unistd.h>
int chdir(const char *);
char *getcwd(char *, size_t);
void usleep(int);

// <pthread.h>
typedef struct opaque_pthread_t {} *pthread_t;
typedef struct opaque_pthread_attr_t {} pthread_attr_t;
int pthread_create(pthread_t * thread,
                          const pthread_attr_t * attr,
                          void *(*start_routine)(void *),
                          void * arg);
int pthread_join(pthread_t thread, void **retval);

// Objective-C base:
typedef signed char BOOL;
#define false 0;
#define true 1;
typedef struct objc_selector* SEL;
typedef struct objc_class* Class;
typedef struct objc_object {
    Class isa;
} *id;

id objc_msgSend(id, SEL, ...);

@interface NSObject {
    Class isa;
}
+(id) new;
-(id) init;

@end
