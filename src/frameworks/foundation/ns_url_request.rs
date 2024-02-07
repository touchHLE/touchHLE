use crate::{
    frameworks::foundation::{NSTimeInterval, NSUInteger},
    objc::{id, ClassExports, HostObject},
    objc_classes,
};

struct NSURLRequestHostObject {}
impl HostObject for NSURLRequestHostObject {}

struct NSURLConnectionHostObject {}
impl HostObject for NSURLConnectionHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

    (env, this, _cmd);

    @implementation NSMutableURLRequest: NSURLRequest

    - (())setURL:(id)_URL {

    }

    - (())setHTTPMethod:(id)_method {

    }


    @end

    @implementation NSURLRequest: NSObject

    + (id)requestWithURL:(id)_URL
                   cachePolicy:(NSUInteger)_cachePolicy
               timeoutInterval:(NSTimeInterval)_timeoutInterval {
        env.objc.alloc_object(this, Box::new(NSURLRequestHostObject{}), &mut env.mem)
    }

    @end

    @implementation NSURLConnection: NSObject

    - (id)initWithRequest:(id)_request
                       delegate:(id)_delegate
               startImmediately:(bool)_startImmediately {
        id::null()
    }

    @end

};
