#ifndef TOUCHHLE_OBJC_SYSTEM_H
#define TOUCHHLE_OBJC_SYSTEM_H

#include "system_headers.h"

// Objective-C base:
typedef signed char BOOL;
#define false 0
#define true 1
typedef struct objc_selector *SEL;
typedef struct objc_class *Class;
typedef struct objc_object {
  Class isa;
} *id;

id objc_msgSend(id, SEL, ...);

@interface NSObject {
  Class isa;
}
+ (id)new;
- (id)init;

@end
#endif // TOUCHHLE_OBJC_SYSTEM_H
