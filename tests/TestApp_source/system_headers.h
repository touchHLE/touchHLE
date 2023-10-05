// === Declarations ===

// We don't have any system headers for iPhone OS, so we must declare everything
// ourselves rather than #include'ing.

#ifndef TOUCHHLE_SYSTEM_H
#define TOUCHHLE_SYSTEM_H

// <stddef.h>
#define NULL ((void *)0)
typedef unsigned long size_t;
typedef int wchar_t;

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
int swprintf(wchar_t *, size_t, const wchar_t *, ...);

// <stdlib.h>
#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1
void exit(int);
void free(void *);
void *malloc(size_t);
void qsort(void *, size_t, size_t, int (*)(const void *, const void *));
void *realloc(void *, size_t);
double atof(const char *);
float strtof(const char *, char **);
unsigned long strtoul(const char *, char **, int);

// <string.h>
void *memset(void *, int, size_t);
int memcmp(const void *, const void *, size_t);
void *memmove(void *, const void *, size_t);
int strcmp(const char *, const char *);
char *strncpy(char *, const char *, size_t);
char *strncat(char *, const char *, size_t);
size_t strlcpy(char *, const char *, size_t);
char *strchr(const char *s, int c);
char *strrchr(const char *s, int c);

// <unistd.h>
typedef unsigned int __uint32_t;
typedef __uint32_t useconds_t;
int chdir(const char *);
char *getcwd(char *, size_t);
int usleep(useconds_t);

// <fcntl.h>
#define O_CREAT 0x00000200

// <pthread.h>
typedef struct opaque_pthread_t opaque_pthread_t;
typedef struct opaque_pthread_t *__pthread_t;
typedef __pthread_t pthread_t;
typedef struct opaque_pthread_attr_t opaque_pthread_attr_t;
typedef struct opaque_pthread_attr_t *__pthread_attr_t;
typedef __pthread_attr_t pthread_attr_t;
int pthread_create(pthread_t *, const pthread_attr_t *, void *(*)(void *),
                   void *);
int pthread_join(pthread_t thread, void **value_ptr);

// <semaphore.h>
#define SEM_FAILED ((sem_t *)-1)
typedef int sem_t;
int sem_close(sem_t *);
sem_t *sem_open(const char *, int, ...);
int sem_post(sem_t *);
int sem_trywait(sem_t *);
int sem_unlink(const char *);
int sem_wait(sem_t *);

// <stdbool.h>
#ifndef __cplusplus
typedef _Bool bool;
#endif

// <locale.h>
#define LC_ALL 0
#define LC_COLLATE 1
#define LC_CTYPE 2
#define LC_MONETARY 3
#define LC_NUMERIC 4
#define LC_TIME 5
#define LC_MESSAGES 6
char *setlocale(int category, const char *locale);

// <dirent.h>
typedef struct {
  int _unused;
} DIR;
struct dirent {
  char _unused[21]; // TODO
  char d_name[1024];
};
DIR *opendir(const char *);
struct dirent *readdir(DIR *);
int closedir(DIR *);

// Stuff from various Core Graphics headers.
#ifdef DEFINE_ME_WHEN_BUILDING_ON_MACOS
typedef double CGFloat; // 64-bit definition (not supported by touchHLE)
#else
typedef float CGFloat;
#endif

typedef struct {
  CGFloat x, y;
} CGPoint;
bool CGPointEqualToPoint(CGPoint, CGPoint);
typedef struct {
  CGFloat width, height;
} CGSize;
bool CGSizeEqualToSize(CGSize, CGSize);
typedef struct {
  CGPoint origin;
  CGSize size;
} CGRect;
bool CGRectEqualToRect(CGRect, CGRect);

typedef struct {
  CGFloat a, b, c, d, tx, ty;
} CGAffineTransform;
// extern const CGAffineTransform CGAffineTransformIdentity;
bool CGAffineTransformIsIdentity(CGAffineTransform);
bool CGAffineTransformEqualToTransform(CGAffineTransform, CGAffineTransform);
CGAffineTransform CGAffineTransformMake(CGFloat, CGFloat, CGFloat, CGFloat,
                                        CGFloat, CGFloat);
CGAffineTransform CGAffineTransformMakeRotation(CGFloat);
CGAffineTransform CGAffineTransformMakeScale(CGFloat, CGFloat);
CGAffineTransform CGAffineTransformMakeTranslation(CGFloat, CGFloat);
CGAffineTransform CGAffineTransformConcat(CGAffineTransform, CGAffineTransform);
CGAffineTransform CGAffineTransformRotate(CGAffineTransform, CGFloat);
CGAffineTransform CGAffineTransformScale(CGAffineTransform, CGFloat, CGFloat);
CGAffineTransform CGAffineTransformTranslate(CGAffineTransform, CGFloat,
                                             CGFloat);
CGAffineTransform CGAffineTransformInvert(CGAffineTransform);
CGPoint CGPointApplyAffineTransform(CGPoint, CGAffineTransform);
CGSize CGSizeApplyAffineTransform(CGSize, CGAffineTransform);
CGRect CGRectApplyAffineTransform(CGRect, CGAffineTransform);

#endif // TOUCHHLE_SYSTEM_H
