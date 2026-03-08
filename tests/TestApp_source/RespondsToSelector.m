#import "system_headers.h"
@implementation TestClass : NSObject {
}
+ (void)classSelector {
}
- (void)instanceSelector {
}
@end

int test_RespondsToSelector(void) {
  TestClass *instance = [TestClass new];
  SEL instance_sel =
      NSSelectorFromString([NSString stringWithUTF8String:"instanceSelector"]);
  SEL class_sel =
      NSSelectorFromString([NSString stringWithUTF8String:"classSelector"]);
  if (![instance respondsToSelector:instance_sel] ||
      [instance respondsToSelector:class_sel]) {
    return -1;
  }
  if ([TestClass respondsToSelector:instance_sel] ||
      ![TestClass respondsToSelector:class_sel]) {
    return -2;
  }
  return 0;
}
