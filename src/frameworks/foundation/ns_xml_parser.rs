/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! NSXMLParser.
//!
//! "Real" iOS implementation probably uses libxml2 under the hood
//! (at least because error codes are identical to libxml,
//! at most because it does make sense).
//!
//! Our implementation is based instead on [quick-xml crate](https://docs.rs/quick-xml/latest/quick_xml) for convenience.
//! This is something to reconsider once we integrate
//! libxml dylib into the project.

use super::ns_string::{from_rust_string, to_rust_string};
use super::NSUInteger;
use crate::environment::Environment;
use crate::mem::ConstVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr, SEL,
};
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

struct NSXMLParserHostObject {
    /// An internal representation of XML data to parse using NSData*
    data: id,
    /// An object conforming to NSXMLParserDelegateEventAdditions category
    /// or NSXMLParserDelegate protocol (which is equivalent)
    delegate: id,
}
impl HostObject for NSXMLParserHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSXMLParser: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSXMLParserHostObject {
        data: nil,
        delegate: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// TODO: more init methods
// TODO: more delegate messages to support

- (id)initWithContentsOfURL:(id)url { // NSURL *
    let data: id = msg_class![env; NSData dataWithContentsOfURL:url];
    msg![env; this initWithData:data]
}

- (id)initWithData:(id)data { // NSData *
    retain(env, data);
    env.objc.borrow_mut::<NSXMLParserHostObject>(this).data = data;
    this
}

// weak/non-retaining
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<NSXMLParserHostObject>(this).delegate = delegate;
}
- (id)delegate {
    env.objc.borrow::<NSXMLParserHostObject>(this).delegate
}

- (())setShouldResolveExternalEntities:(bool)should {
    log_dbg!("TODO: setShouldResolveExternalEntities:{}", should);
}

- (bool)parse {
    let data = env.objc.borrow::<NSXMLParserHostObject>(this).data;
    assert_ne!(data, nil);
    let bytes: ConstVoidPtr = msg![env; data bytes];
    let length: NSUInteger = msg![env; data length];
    log_dbg!("Parsing {:?}", env.mem.cstr_at_utf8(bytes.cast()));
    let bytes: &[u8] = env.mem.bytes_at_mut(bytes.cast().cast_mut(), length);

    // TODO: support partial parsing of XML
    let mut reader = Reader::from_reader(bytes);
    // TODO: parse and send delegate messages in one pass
    let mut events = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(e) => events.push(e.into_owned()), // TODO: avoid copying
            Err(e) => {
                // TODO: send parser:parseErrorOccurred: to delegate instead,
                // after (!) other parsing delegate messages were sent
                panic!("Error at position {}: {:?}", reader.error_position(), e)
            },
        }
    }

    let delegate = env.objc.borrow::<NSXMLParserHostObject>(this).delegate;
    let sel: SEL = env
        .objc
        .register_host_selector("parserDidStartDocument:".to_string(), &mut env.mem);
    let responds: bool = msg![env; delegate respondsToSelector:sel];
    if responds {
        () = msg![env; delegate parserDidStartDocument:this];
    }
    for event in events {
        match event {
            Event::Empty(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let name: id = from_rust_string(env, name);
                let name = autorelease(env, name);
                let sel: SEL = env
                    .objc
                    .register_host_selector(
                        "parser:didStartElement:namespaceURI:qualifiedName:attributes:".to_string(),
                        &mut env.mem
                    );
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    let dict = build_attributes_dict(env, e);
                    () = msg![env; delegate parser:this
                                   didStartElement:name
                                      namespaceURI:nil
                                     qualifiedName:nil
                                        attributes:dict];
                }
                let sel: SEL = env
                    .objc
                    .register_host_selector(
                        "parser:didEndElement:namespaceURI:qualifiedName:".to_string(),
                        &mut env.mem
                    );
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    () = msg![env; delegate parser:this
                                     didEndElement:name
                                      namespaceURI:nil
                                     qualifiedName:nil];
                }
            }
            Event::Text(e) => {
                let text = e.unescape().unwrap().into_owned();
                // FIXME: skipping the end of the parsed string?
                if text != "\0" {
                    let sel: SEL = env
                        .objc
                        .register_host_selector("parser:foundCharacters:".to_string(), &mut env.mem);
                    let responds: bool = msg![env; delegate respondsToSelector:sel];
                    if responds {
                        let chars = from_rust_string(env, text);
                        let chars = autorelease(env, chars);
                        () = msg![env; delegate parser:this foundCharacters:chars];
                    }
                }
            }
            Event::Start(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let sel: SEL = env
                    .objc
                    .register_host_selector(
                        "parser:didStartElement:namespaceURI:qualifiedName:attributes:".to_string(),
                        &mut env.mem
                    );
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    let name: id = from_rust_string(env, name);
                    let name = autorelease(env, name);
                    let dict = build_attributes_dict(env, e);
                    () = msg![env; delegate parser:this
                                   didStartElement:name
                                      namespaceURI:nil
                                     qualifiedName:nil
                                        attributes:dict];
                }
            }
            Event::End(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let sel: SEL = env
                    .objc
                    .register_host_selector(
                        "parser:didEndElement:namespaceURI:qualifiedName:".to_string(),
                        &mut env.mem
                    );
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    let name: id = from_rust_string(env, name);
                    let name = autorelease(env, name);
                    () = msg![env; delegate parser:this
                                     didEndElement:name
                                      namespaceURI:nil
                                     qualifiedName:nil];
                }
            }
            Event::CData(e) => {
                let sel: SEL = env
                    .objc
                    .register_host_selector("parser:foundCDATA:".to_string(), &mut env.mem);
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    todo!("Implement parser:foundCDATA: delegate call");
                } else {
                    let sel: SEL = env
                        .objc
                        .register_host_selector("parser:foundCharacters:".to_string(), &mut env.mem);
                    let responds: bool = msg![env; delegate respondsToSelector:sel];
                    if responds {
                        let text = e.escape().unwrap().unescape().unwrap().to_string();
                        let text = from_rust_string(env, text);
                        let text = autorelease(env, text);
                        () = msg![env; delegate parser:this foundCharacters:text];
                    }
                }
            }
            Event::Comment(e) => {
                let comment = e.unescape().unwrap().into_owned();
                let sel: SEL = env
                    .objc
                    .register_host_selector("parser:foundComment:".to_string(), &mut env.mem);
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    let comment = from_rust_string(env, comment);
                    let comment = autorelease(env, comment);
                    () = msg![env; delegate parser:this foundComment:comment];
                }
            }
            e => unimplemented!("{:?}", e)
        }
    }
    let sel: SEL = env
        .objc
        .register_host_selector("parserDidEndDocument:".to_string(), &mut env.mem);
    let responds: bool = msg![env; delegate respondsToSelector:sel];
    if responds {
        () = msg![env; delegate parserDidEndDocument:this];
    }
    true
}

- (())dealloc {
    let &NSXMLParserHostObject { data, .. } = env.objc.borrow(this);
    release(env, data);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};

/// A helper function to build an attributes NSDictionary from an XML tag.
/// Each key/value pair is copied and retained in the dict.
fn build_attributes_dict(env: &mut Environment, e: BytesStart) -> id {
    let pairs = e.attributes().map(|a| a.unwrap()).map(|a| {
        (
            String::from_utf8(a.key.local_name().as_ref().to_vec()).unwrap(),
            a.unescape_value().unwrap().to_string(),
        )
    });
    let dict: id = msg_class![env; NSMutableDictionary new];
    for (x, y) in pairs {
        let key = from_rust_string(env, x);
        let val = from_rust_string(env, y);
        () = msg![env; dict setObject:val forKey:key];
        release(env, key);
        release(env, val);
    }
    log_dbg!("attributes {}", {
        let desc = msg![env; dict description];
        to_rust_string(env, desc)
    });
    // TODO: return an immutable copy
    autorelease(env, dict)
}
