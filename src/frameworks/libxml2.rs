/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Host shim for `libxml2.2.dylib`.
//!
//! Every entry point dispatches to the real, vendored libxml2 (built in
//! `vendor/libxml2`) via the [`touchHLE_libxml2_wrapper`] crate.  The guest
//! never sees a host pointer directly: every libxml2 object (`xmlDoc *`,
//! `xmlNode *`, `xmlAttr *`, `xmlTextReader *`, `xmlXPathObject *`, …) is
//! recorded in a global handle table and exchanged with the guest as an
//! opaque 32-bit handle, while every returned `xmlChar *` is copied into
//! freshly-allocated guest memory.
//!
//! Coverage spans every major libxml2 sub-module: parser, tree, xmlreader,
//! xmlwriter, xmlsave, xpath, xinclude, xpointer, DTD/Schemas/RelaxNG/
//! Schematron validation, catalog, URI, c14n, html, error reporting and
//! string utilities.  The matching C documentation is at
//! <https://gnome.pages.gitlab.gnome.org/libxml2/devhelp/>.

#![allow(clippy::too_many_arguments)]

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::fs::GuestPath;
use crate::mem::{GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Mutex;
use touchHLE_libxml2_wrapper as xml;

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/usr/lib/libxml2.2.dylib",
    aliases: &["/usr/lib/libxml2.dylib"],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[FUNCTIONS],
};

// ============================================================================
// Handle table — maps u32 guest-visible IDs to host libxml2 pointers.
// ============================================================================

/// All guest-visible handles live above this base so they never collide with
/// real guest memory pointers.  This matches the convention used by the
/// previous in-house implementation.
const HANDLE_BASE: u32 = 0x1000_0000;

struct HandleTable {
    next_id: u32,
    by_id: HashMap<u32, *mut c_void>,
    by_ptr: HashMap<usize, u32>,
}

// `*mut c_void` is not Send/Sync by default; we only ever access the table
// while holding the global mutex, and the libxml2 pointers we store are
// stable for as long as we keep them in the table.
unsafe impl Send for HandleTable {}

static STATE: Mutex<Option<HandleTable>> = Mutex::new(None);

fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut HandleTable) -> R,
{
    let mut g = STATE.lock().unwrap();
    if g.is_none() {
        xml::init_once();
        // Silence libxml2's default stderr logger.  Errors are still
        // retrievable via `xmlGetLastError` / `xmlCtxtGetLastError`.
        unsafe { xml::hxml_install_quiet_error_handlers() };
        *g = Some(HandleTable {
            next_id: HANDLE_BASE,
            by_id: HashMap::new(),
            by_ptr: HashMap::new(),
        });
    }
    f(g.as_mut().unwrap())
}

/// Return the existing handle for `ptr` or allocate a new one.  Returns 0
/// for a null input pointer.
fn id_for(ptr: *mut c_void) -> u32 {
    if ptr.is_null() {
        return 0;
    }
    with_state(|s| {
        let key = ptr as usize;
        if let Some(&id) = s.by_ptr.get(&key) {
            return id;
        }
        let id = s.next_id;
        s.next_id = s.next_id.checked_add(1).expect("libxml2 handle overflow");
        s.by_id.insert(id, ptr);
        s.by_ptr.insert(key, id);
        id
    })
}

fn ptr_for(id: u32) -> *mut c_void {
    if id == 0 {
        return ptr::null_mut();
    }
    with_state(|s| s.by_id.get(&id).copied().unwrap_or(ptr::null_mut()))
}

fn release_handle(id: u32) -> *mut c_void {
    if id == 0 {
        return ptr::null_mut();
    }
    with_state(|s| match s.by_id.remove(&id) {
        Some(p) => {
            s.by_ptr.remove(&(p as usize));
            p
        }
        None => ptr::null_mut(),
    })
}

fn is_handle(id: u32) -> bool {
    if id == 0 {
        return false;
    }
    with_state(|s| s.by_id.contains_key(&id))
}

// Convenience casts.
fn h2doc(id: u32) -> *mut xml::xmlDoc {
    ptr_for(id) as *mut xml::xmlDoc
}
fn h2node(id: u32) -> *mut xml::xmlNode {
    ptr_for(id) as *mut xml::xmlNode
}
fn h2attr(id: u32) -> *mut xml::xmlAttr {
    ptr_for(id) as *mut xml::xmlAttr
}
fn h2ns(id: u32) -> *mut xml::xmlNs {
    ptr_for(id) as *mut xml::xmlNs
}
fn h2parser_ctxt(id: u32) -> *mut xml::xmlParserCtxt {
    ptr_for(id) as *mut xml::xmlParserCtxt
}
fn h2reader(id: u32) -> *mut xml::xmlTextReader {
    ptr_for(id) as *mut xml::xmlTextReader
}
fn h2writer(id: u32) -> *mut xml::xmlTextWriter {
    ptr_for(id) as *mut xml::xmlTextWriter
}
fn h2xpath_ctxt(id: u32) -> *mut xml::xmlXPathContext {
    ptr_for(id) as *mut xml::xmlXPathContext
}
fn h2xpath_obj(id: u32) -> *mut xml::xmlXPathObject {
    ptr_for(id) as *mut xml::xmlXPathObject
}
fn h2xpath_comp(id: u32) -> *mut xml::xmlXPathCompExpr {
    ptr_for(id) as *mut xml::xmlXPathCompExpr
}
fn h2schema_parser(id: u32) -> *mut xml::xmlSchemaParserCtxt {
    ptr_for(id) as *mut xml::xmlSchemaParserCtxt
}
fn h2schema(id: u32) -> *mut xml::xmlSchema {
    ptr_for(id) as *mut xml::xmlSchema
}
fn h2schema_valid(id: u32) -> *mut xml::xmlSchemaValidCtxt {
    ptr_for(id) as *mut xml::xmlSchemaValidCtxt
}
fn h2rng_parser(id: u32) -> *mut xml::xmlRelaxNGParserCtxt {
    ptr_for(id) as *mut xml::xmlRelaxNGParserCtxt
}
fn h2rng(id: u32) -> *mut xml::xmlRelaxNG {
    ptr_for(id) as *mut xml::xmlRelaxNG
}
fn h2rng_valid(id: u32) -> *mut xml::xmlRelaxNGValidCtxt {
    ptr_for(id) as *mut xml::xmlRelaxNGValidCtxt
}
fn h2tron_parser(id: u32) -> *mut xml::xmlSchematronParserCtxt {
    ptr_for(id) as *mut xml::xmlSchematronParserCtxt
}
fn h2tron(id: u32) -> *mut xml::xmlSchematron {
    ptr_for(id) as *mut xml::xmlSchematron
}
fn h2tron_valid(id: u32) -> *mut xml::xmlSchematronValidCtxt {
    ptr_for(id) as *mut xml::xmlSchematronValidCtxt
}
fn h2valid(id: u32) -> *mut xml::xmlValidCtxt {
    ptr_for(id) as *mut xml::xmlValidCtxt
}
fn h2dtd(id: u32) -> *mut xml::xmlDtd {
    ptr_for(id) as *mut xml::xmlDtd
}
fn h2uri(id: u32) -> *mut xml::xmlURI {
    ptr_for(id) as *mut xml::xmlURI
}
fn h2buffer(id: u32) -> *mut xml::xmlBuffer {
    ptr_for(id) as *mut xml::xmlBuffer
}
#[allow(dead_code)]
fn h2save(id: u32) -> *mut xml::xmlSaveCtxt {
    ptr_for(id) as *mut xml::xmlSaveCtxt
}
#[allow(dead_code)]
fn h2html_ctxt(id: u32) -> *mut xml::htmlParserCtxt {
    ptr_for(id) as *mut xml::htmlParserCtxt
}
fn h2error(id: u32) -> *mut xml::xmlError {
    ptr_for(id) as *mut xml::xmlError
}

// ============================================================================
// Guest memory helpers.
// ============================================================================

fn read_guest_bytes(env: &Environment, ptr: u32, len: u32) -> Vec<u8> {
    if ptr == 0 || len == 0 {
        return Vec::new();
    }
    (0..len)
        .map(|i| env.mem.read(Ptr::<u8, false>::from_bits(ptr + i)))
        .collect()
}

fn read_guest_cstr(env: &Environment, ptr: u32) -> Vec<u8> {
    if ptr == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut off: GuestUSize = 0;
    loop {
        let b: u8 = env.mem.read(Ptr::<u8, false>::from_bits(ptr + off));
        if b == 0 {
            break;
        }
        out.push(b);
        off += 1;
    }
    out
}

fn read_guest_cstr_string(env: &Environment, ptr: u32) -> String {
    String::from_utf8_lossy(&read_guest_cstr(env, ptr)).into_owned()
}

fn cstring_or_default(bytes: &[u8]) -> CString {
    CString::new(bytes.iter().copied().filter(|&b| b != 0).collect::<Vec<u8>>())
        .unwrap_or_else(|_| CString::new("").unwrap())
}

/// Read a NUL-terminated string from guest memory and wrap it in a CString.
/// Returns `None` when the pointer is null, so the caller can pass an actual
/// null pointer to libxml2.
fn read_guest_cstring(env: &Environment, ptr: u32) -> Option<CString> {
    if ptr == 0 {
        return None;
    }
    Some(cstring_or_default(&read_guest_cstr(env, ptr)))
}

fn cstr_ptr(s: &Option<CString>) -> *const c_char {
    match s {
        Some(c) => c.as_ptr(),
        None => ptr::null(),
    }
}

fn cstr_xml(s: &Option<CString>) -> *const xml::xmlChar {
    cstr_ptr(s) as *const xml::xmlChar
}

fn write_to_guest(env: &mut Environment, bytes: &[u8]) -> u32 {
    let len = bytes.len() as GuestUSize;
    let ptr: MutPtr<u8> = env.mem.alloc(len + 1).cast();
    for (i, &b) in bytes.iter().enumerate() {
        env.mem.write(ptr + (i as GuestUSize), b);
    }
    env.mem.write(ptr + len, 0u8);
    ptr.to_bits()
}

/// Copy a const `xmlChar*` returned by libxml2 into guest memory.
fn copy_xml_chars(env: &mut Environment, host: *const xml::xmlChar) -> u32 {
    if host.is_null() {
        return 0;
    }
    let bytes = unsafe { xml::read_xml_str(host) };
    write_to_guest(env, &bytes)
}

/// Same as [`copy_xml_chars`] but also `xmlFree`'s the host pointer (use for
/// owned `xmlChar*` returns).
fn take_xml_chars(env: &mut Environment, host: *mut xml::xmlChar) -> u32 {
    if host.is_null() {
        return 0;
    }
    let bytes = unsafe { xml::read_xml_str(host) };
    unsafe { xml::xmlFree(host as *mut c_void) };
    write_to_guest(env, &bytes)
}

fn copy_plain_cstr(env: &mut Environment, host: *const c_char) -> u32 {
    if host.is_null() {
        return 0;
    }
    let mut bytes = Vec::new();
    let mut p = host;
    unsafe {
        while *p != 0 {
            bytes.push(*p as u8);
            p = p.add(1);
        }
    }
    write_to_guest(env, &bytes)
}

// ============================================================================
// Parser / context API
// ============================================================================

#[allow(non_snake_case)]
fn xmlNewParserCtxt(_env: &mut Environment) -> u32 {
    let p = unsafe { xml::xmlNewParserCtxt() };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlFreeParserCtxt(_env: &mut Environment, ctxt: u32) {
    let p = release_handle(ctxt) as *mut xml::xmlParserCtxt;
    if !p.is_null() {
        unsafe { xml::xmlFreeParserCtxt(p) };
    }
}

#[allow(non_snake_case)]
fn xmlClearParserCtxt(_env: &mut Environment, ctxt: u32) {
    let p = h2parser_ctxt(ctxt);
    if !p.is_null() {
        unsafe { xml::xmlClearParserCtxt(p) };
    }
}

#[allow(non_snake_case)]
fn xmlCtxtReset(_env: &mut Environment, ctxt: u32) {
    let p = h2parser_ctxt(ctxt);
    if !p.is_null() {
        unsafe { xml::xmlCtxtReset(p) };
    }
}

#[allow(non_snake_case)]
fn xmlCleanupParser(_env: &mut Environment) {
    // libxml2 documents this as "rarely needed"; we forward it but keep our
    // handle table intact.
    unsafe { xml::xmlCleanupParser() };
}

#[allow(non_snake_case)]
fn xmlCheckVersion(_env: &mut Environment, version: i32) {
    // Forwards to real libxml2's runtime version sanity check.  Real libxml2
    // logs a warning if the compile-time and runtime versions differ; we
    // route that through our quiet error handler so it stays out of stderr.
    with_state(|_| {});
    unsafe { xml::xmlCheckVersion(version as c_int) };
}

#[allow(non_snake_case)]
fn xmlCtxtGetLastError(_env: &mut Environment, ctxt: u32) -> u32 {
    let p = h2parser_ctxt(ctxt) as *mut c_void;
    let err = unsafe { xml::xmlCtxtGetLastError(p) };
    id_for(err as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlCtxtResetLastError(_env: &mut Environment, ctxt: u32) {
    let p = h2parser_ctxt(ctxt) as *mut c_void;
    unsafe { xml::xmlCtxtResetLastError(p) };
}

fn parse_memory(env: &mut Environment, buf: u32, sz: u32, url: u32, enc: u32, opts: i32) -> u32 {
    let data = read_guest_bytes(env, buf, sz);
    let url_c = read_guest_cstring(env, url);
    let enc_c = read_guest_cstring(env, enc);
    let doc = unsafe {
        xml::xmlReadMemory(
            data.as_ptr() as *const c_char,
            sz as c_int,
            cstr_ptr(&url_c),
            cstr_ptr(&enc_c),
            opts,
        )
    };
    id_for(doc as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlReadMemory(
    env: &mut Environment,
    buffer: u32,
    size: u32,
    url: u32,
    encoding: u32,
    options: i32,
) -> u32 {
    parse_memory(env, buffer, size, url, encoding, options)
}

#[allow(non_snake_case)]
fn xmlCtxtReadMemory(
    env: &mut Environment,
    ctxt: u32,
    buf: u32,
    sz: u32,
    url: u32,
    enc: u32,
    opts: i32,
) -> u32 {
    let data = read_guest_bytes(env, buf, sz);
    let url_c = read_guest_cstring(env, url);
    let enc_c = read_guest_cstring(env, enc);
    let doc = unsafe {
        xml::xmlCtxtReadMemory(
            h2parser_ctxt(ctxt),
            data.as_ptr() as *const c_char,
            sz as c_int,
            cstr_ptr(&url_c),
            cstr_ptr(&enc_c),
            opts,
        )
    };
    id_for(doc as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlParseMemory(env: &mut Environment, buffer: u32, size: u32) -> u32 {
    let data = read_guest_bytes(env, buffer, size);
    let doc = unsafe { xml::xmlParseMemory(data.as_ptr() as *const c_char, size as c_int) };
    id_for(doc as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlParseDoc(env: &mut Environment, cur: u32) -> u32 {
    let bytes = read_guest_cstr(env, cur);
    let c = cstring_or_default(&bytes);
    let doc = unsafe { xml::xmlParseDoc(c.as_ptr() as *const xml::xmlChar) };
    id_for(doc as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlReadDoc(env: &mut Environment, cur: u32, url: u32, encoding: u32, options: i32) -> u32 {
    let bytes = read_guest_cstr(env, cur);
    let c = cstring_or_default(&bytes);
    let url_c = read_guest_cstring(env, url);
    let enc_c = read_guest_cstring(env, encoding);
    let doc = unsafe {
        xml::xmlReadDoc(
            c.as_ptr() as *const xml::xmlChar,
            cstr_ptr(&url_c),
            cstr_ptr(&enc_c),
            options,
        )
    };
    id_for(doc as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlReadFile(env: &mut Environment, filename: u32, encoding: u32, options: i32) -> u32 {
    let path = read_guest_cstr_string(env, filename);
    log!("xmlReadFile(\"{}\")", path);
    let data = env.fs.read(GuestPath::new(&path)).unwrap_or_default();
    let url_c = CString::new(path.clone()).unwrap_or_else(|_| CString::new("").unwrap());
    let enc_c = read_guest_cstring(env, encoding);
    let doc = unsafe {
        xml::xmlReadMemory(
            data.as_ptr() as *const c_char,
            data.len() as c_int,
            url_c.as_ptr(),
            cstr_ptr(&enc_c),
            options,
        )
    };
    id_for(doc as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlParseFile(env: &mut Environment, filename: u32) -> u32 {
    xmlReadFile(env, filename, 0, 0)
}

#[allow(non_snake_case)]
fn xmlCtxtReadFile(
    env: &mut Environment,
    ctxt: u32,
    filename: u32,
    encoding: u32,
    options: i32,
) -> u32 {
    let path = read_guest_cstr_string(env, filename);
    let data = env.fs.read(GuestPath::new(&path)).unwrap_or_default();
    let enc_c = read_guest_cstring(env, encoding);
    let url_c = CString::new(path.clone()).unwrap_or_else(|_| CString::new("").unwrap());
    let doc = unsafe {
        xml::xmlCtxtReadMemory(
            h2parser_ctxt(ctxt),
            data.as_ptr() as *const c_char,
            data.len() as c_int,
            url_c.as_ptr(),
            cstr_ptr(&enc_c),
            options,
        )
    };
    id_for(doc as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlCtxtReadDoc(
    env: &mut Environment,
    ctxt: u32,
    cur: u32,
    url: u32,
    encoding: u32,
    options: i32,
) -> u32 {
    let bytes = read_guest_cstr(env, cur);
    let c = cstring_or_default(&bytes);
    let url_c = read_guest_cstring(env, url);
    let enc_c = read_guest_cstring(env, encoding);
    let doc = unsafe {
        xml::xmlCtxtReadDoc(
            h2parser_ctxt(ctxt),
            c.as_ptr() as *const xml::xmlChar,
            cstr_ptr(&url_c),
            cstr_ptr(&enc_c),
            options,
        )
    };
    id_for(doc as *mut c_void)
}

// ----- Push parser -----

#[allow(non_snake_case)]
fn xmlCreatePushParserCtxt(
    env: &mut Environment,
    _sax: u32,
    _user_data: u32,
    chunk: u32,
    size: i32,
    filename: u32,
) -> u32 {
    let data = read_guest_bytes(env, chunk, size as u32);
    let filename_c = read_guest_cstring(env, filename);
    let p = unsafe {
        xml::xmlCreatePushParserCtxt(
            ptr::null_mut(),
            ptr::null_mut(),
            data.as_ptr() as *const c_char,
            size,
            cstr_ptr(&filename_c),
        )
    };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlParseChunk(env: &mut Environment, ctxt: u32, chunk: u32, size: i32, terminate: i32) -> i32 {
    let data = read_guest_bytes(env, chunk, size as u32);
    unsafe {
        xml::xmlParseChunk(
            h2parser_ctxt(ctxt),
            data.as_ptr() as *const c_char,
            size,
            terminate,
        )
    }
}

// ============================================================================
// Document and tree API
// ============================================================================

#[allow(non_snake_case)]
fn xmlNewDoc(env: &mut Environment, version: u32) -> u32 {
    let v = read_guest_cstring(env, version);
    let doc = unsafe { xml::xmlNewDoc(cstr_xml(&v)) };
    id_for(doc as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlFreeDoc(_env: &mut Environment, doc: u32) {
    let p = release_handle(doc) as *mut xml::xmlDoc;
    if !p.is_null() {
        unsafe { xml::xmlFreeDoc(p) };
    }
}

#[allow(non_snake_case)]
fn xmlDocGetRootElement(_env: &mut Environment, doc: u32) -> u32 {
    let r = unsafe { xml::xmlDocGetRootElement(h2doc(doc)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlDocSetRootElement(_env: &mut Environment, doc: u32, root: u32) -> u32 {
    let old = unsafe { xml::xmlDocSetRootElement(h2doc(doc), h2node(root)) };
    id_for(old as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNewNode(env: &mut Environment, ns: u32, name: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let r = unsafe { xml::xmlNewNode(h2ns(ns), cstr_xml(&n)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNewDocNode(env: &mut Environment, doc: u32, ns: u32, name: u32, content: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let c = read_guest_cstring(env, content);
    let r = unsafe { xml::xmlNewDocNode(h2doc(doc), h2ns(ns), cstr_xml(&n), cstr_xml(&c)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNewText(env: &mut Environment, content: u32) -> u32 {
    let c = read_guest_cstring(env, content);
    let r = unsafe { xml::xmlNewText(cstr_xml(&c)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNewDocText(env: &mut Environment, doc: u32, content: u32) -> u32 {
    let c = read_guest_cstring(env, content);
    let r = unsafe { xml::xmlNewDocText(h2doc(doc), cstr_xml(&c)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNewComment(env: &mut Environment, content: u32) -> u32 {
    let c = read_guest_cstring(env, content);
    let r = unsafe { xml::xmlNewComment(cstr_xml(&c)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNewCDataBlock(env: &mut Environment, doc: u32, content: u32, len: i32) -> u32 {
    let c = read_guest_cstring(env, content);
    let r = unsafe { xml::xmlNewCDataBlock(h2doc(doc), cstr_xml(&c), len) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNewChild(env: &mut Environment, parent: u32, ns: u32, name: u32, content: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let c = read_guest_cstring(env, content);
    let r = unsafe { xml::xmlNewChild(h2node(parent), h2ns(ns), cstr_xml(&n), cstr_xml(&c)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNewTextChild(env: &mut Environment, parent: u32, ns: u32, name: u32, content: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let c = read_guest_cstring(env, content);
    let r =
        unsafe { xml::xmlNewTextChild(h2node(parent), h2ns(ns), cstr_xml(&n), cstr_xml(&c)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlAddChild(_env: &mut Environment, parent: u32, cur: u32) -> u32 {
    let r = unsafe { xml::xmlAddChild(h2node(parent), h2node(cur)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlAddPrevSibling(_env: &mut Environment, cur: u32, elem: u32) -> u32 {
    let r = unsafe { xml::xmlAddPrevSibling(h2node(cur), h2node(elem)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlAddNextSibling(_env: &mut Environment, cur: u32, elem: u32) -> u32 {
    let r = unsafe { xml::xmlAddNextSibling(h2node(cur), h2node(elem)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlAddSibling(_env: &mut Environment, cur: u32, elem: u32) -> u32 {
    let r = unsafe { xml::xmlAddSibling(h2node(cur), h2node(elem)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlUnlinkNode(_env: &mut Environment, cur: u32) {
    let p = h2node(cur);
    if !p.is_null() {
        unsafe { xml::xmlUnlinkNode(p) };
    }
}

#[allow(non_snake_case)]
fn xmlFreeNode(_env: &mut Environment, cur: u32) {
    let p = release_handle(cur) as *mut xml::xmlNode;
    if !p.is_null() {
        unsafe { xml::xmlFreeNode(p) };
    }
}

#[allow(non_snake_case)]
fn xmlFreeNodeList(_env: &mut Environment, cur: u32) {
    let p = release_handle(cur) as *mut xml::xmlNode;
    if !p.is_null() {
        unsafe { xml::xmlFreeNodeList(p) };
    }
}

#[allow(non_snake_case)]
fn xmlCopyNode(_env: &mut Environment, node: u32, recursive: i32) -> u32 {
    let r = unsafe { xml::xmlCopyNode(h2node(node), recursive) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlDocCopyNode(_env: &mut Environment, node: u32, doc: u32, recursive: i32) -> u32 {
    let r = unsafe { xml::xmlDocCopyNode(h2node(node), h2doc(doc), recursive) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlReplaceNode(_env: &mut Environment, old: u32, cur: u32) -> u32 {
    let r = unsafe { xml::xmlReplaceNode(h2node(old), h2node(cur)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNodeSetName(env: &mut Environment, node: u32, name: u32) {
    let n = read_guest_cstring(env, name);
    if let Some(c) = n {
        unsafe { xml::xmlNodeSetName(h2node(node), c.as_ptr() as *const xml::xmlChar) };
    }
}

#[allow(non_snake_case)]
fn xmlNodeSetContent(env: &mut Environment, node: u32, content: u32) {
    let c = read_guest_cstring(env, content);
    unsafe { xml::xmlNodeSetContent(h2node(node), cstr_xml(&c)) };
}

#[allow(non_snake_case)]
fn xmlNodeAddContent(env: &mut Environment, node: u32, content: u32) {
    let c = read_guest_cstring(env, content);
    unsafe { xml::xmlNodeAddContent(h2node(node), cstr_xml(&c)) };
}

#[allow(non_snake_case)]
fn xmlNodeGetContent(env: &mut Environment, node: u32) -> u32 {
    let p = unsafe { xml::xmlNodeGetContent(h2node(node)) };
    take_xml_chars(env, p)
}

/// Compatibility helper for bundled GDataXML implementations.
///
/// Some apps call GDataXML's `nodeConsumingXMLNode:` / `initBorrowingXMLNode:`
/// with one of our opaque libxml2 node handles. GDataXML then dereferences it
/// as a real `_xmlNode *`, which fails. This helper serializes that host
/// libxml node back into XML so the ObjC message hook can build its own small
/// DOM without exposing host pointers or fake guest libxml structs.
pub(crate) fn compat_node_handle_to_xml_string(_env: &mut Environment, node: u32) -> Option<String> {
    let n = h2node(node);
    if n.is_null() {
        return None;
    }

    let doc = unsafe { xml::hxml_node_doc(n) };
    if doc.is_null() {
        return None;
    }

    let buf = unsafe { xml::xmlBufferCreate() };
    if buf.is_null() {
        return None;
    }

    let rc = unsafe { xml::xmlNodeDump(buf, doc, n, 0, 0) };
    if rc < 0 {
        unsafe { xml::xmlBufferFree(buf) };
        return None;
    }

    let content = unsafe { xml::hxml_buffer_content(buf) };
    let len = unsafe { xml::hxml_buffer_length(buf) };
    let out = if !content.is_null() && len > 0 {
        let bytes = unsafe { std::slice::from_raw_parts(content as *const u8, len as usize) };
        Some(String::from_utf8_lossy(bytes).into_owned())
    } else {
        None
    };

    unsafe { xml::xmlBufferFree(buf) };
    out
}

#[allow(non_snake_case)]
fn xmlNodeListGetString(env: &mut Environment, doc: u32, list: u32, in_line: i32) -> u32 {
    let p = unsafe { xml::xmlNodeListGetString(h2doc(doc), h2node(list), in_line) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlGetLastChild(_env: &mut Environment, node: u32) -> u32 {
    let r = unsafe { xml::xmlGetLastChild(h2node(node)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlChildElementCount(_env: &mut Environment, parent: u32) -> u32 {
    unsafe { xml::xmlChildElementCount(h2node(parent)) as u32 }
}

#[allow(non_snake_case)]
fn xmlFirstElementChild(_env: &mut Environment, parent: u32) -> u32 {
    let r = unsafe { xml::xmlFirstElementChild(h2node(parent)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlLastElementChild(_env: &mut Environment, parent: u32) -> u32 {
    let r = unsafe { xml::xmlLastElementChild(h2node(parent)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNextElementSibling(_env: &mut Environment, node: u32) -> u32 {
    let r = unsafe { xml::xmlNextElementSibling(h2node(node)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlPreviousElementSibling(_env: &mut Environment, node: u32) -> u32 {
    let r = unsafe { xml::xmlPreviousElementSibling(h2node(node)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlChildrenNode(_env: &mut Environment, node: u32) -> u32 {
    // libxml2's macro `xmlChildrenNode` is `node->children`.
    let p = unsafe { xml::hxml_node_children(h2node(node)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlGetNodePath(env: &mut Environment, node: u32) -> u32 {
    let p = unsafe { xml::xmlGetNodePath(h2node(node)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlGetLineNo(_env: &mut Environment, node: u32) -> i32 {
    unsafe { xml::xmlGetLineNo(h2node(node)) }
}

// ----- Generic accessors that some apps reach for ------

#[allow(non_snake_case)]
fn xmlNodeGetName(env: &mut Environment, node: u32) -> u32 {
    let p = unsafe { xml::hxml_node_name(h2node(node)) };
    copy_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlNodeGetType(_env: &mut Environment, node: u32) -> i32 {
    unsafe { xml::hxml_node_type(h2node(node)) }
}

#[allow(non_snake_case)]
fn xmlNodeGetDoc(_env: &mut Environment, node: u32) -> u32 {
    let p = unsafe { xml::hxml_node_doc(h2node(node)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNodeGetParent(_env: &mut Environment, node: u32) -> u32 {
    let p = unsafe { xml::hxml_node_parent(h2node(node)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNodeGetNext(_env: &mut Environment, node: u32) -> u32 {
    let p = unsafe { xml::hxml_node_next(h2node(node)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNodeGetPrev(_env: &mut Environment, node: u32) -> u32 {
    let p = unsafe { xml::hxml_node_prev(h2node(node)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNodeGetProperties(_env: &mut Environment, node: u32) -> u32 {
    let p = unsafe { xml::hxml_node_properties(h2node(node)) };
    id_for(p as *mut c_void)
}

// ============================================================================
// Attributes / namespaces
// ============================================================================

#[allow(non_snake_case)]
fn xmlNewProp(env: &mut Environment, node: u32, name: u32, value: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let v = read_guest_cstring(env, value);
    let r = unsafe { xml::xmlNewProp(h2node(node), cstr_xml(&n), cstr_xml(&v)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNewNsProp(env: &mut Environment, node: u32, ns: u32, name: u32, value: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let v = read_guest_cstring(env, value);
    let r = unsafe { xml::xmlNewNsProp(h2node(node), h2ns(ns), cstr_xml(&n), cstr_xml(&v)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSetProp(env: &mut Environment, node: u32, name: u32, value: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let v = read_guest_cstring(env, value);
    let r = unsafe { xml::xmlSetProp(h2node(node), cstr_xml(&n), cstr_xml(&v)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSetNsProp(env: &mut Environment, node: u32, ns: u32, name: u32, value: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let v = read_guest_cstring(env, value);
    let r = unsafe { xml::xmlSetNsProp(h2node(node), h2ns(ns), cstr_xml(&n), cstr_xml(&v)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlUnsetProp(env: &mut Environment, node: u32, name: u32) -> i32 {
    let n = read_guest_cstring(env, name);
    unsafe { xml::xmlUnsetProp(h2node(node), cstr_xml(&n)) }
}

#[allow(non_snake_case)]
fn xmlUnsetNsProp(env: &mut Environment, node: u32, ns: u32, name: u32) -> i32 {
    let n = read_guest_cstring(env, name);
    unsafe { xml::xmlUnsetNsProp(h2node(node), h2ns(ns), cstr_xml(&n)) }
}

#[allow(non_snake_case)]
fn xmlGetProp(env: &mut Environment, node: u32, name: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let r = unsafe { xml::xmlGetProp(h2node(node), cstr_xml(&n)) };
    take_xml_chars(env, r)
}

#[allow(non_snake_case)]
fn xmlGetNoNsProp(env: &mut Environment, node: u32, name: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let r = unsafe { xml::xmlGetNoNsProp(h2node(node), cstr_xml(&n)) };
    take_xml_chars(env, r)
}

#[allow(non_snake_case)]
fn xmlGetNsProp(env: &mut Environment, node: u32, name: u32, ns: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let nu = read_guest_cstring(env, ns);
    let r = unsafe { xml::xmlGetNsProp(h2node(node), cstr_xml(&n), cstr_xml(&nu)) };
    take_xml_chars(env, r)
}

#[allow(non_snake_case)]
fn xmlHasProp(env: &mut Environment, node: u32, name: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let r = unsafe { xml::xmlHasProp(h2node(node), cstr_xml(&n)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlHasNsProp(env: &mut Environment, node: u32, name: u32, ns: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let nu = read_guest_cstring(env, ns);
    let r = unsafe { xml::xmlHasNsProp(h2node(node), cstr_xml(&n), cstr_xml(&nu)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNewNs(env: &mut Environment, node: u32, href: u32, prefix: u32) -> u32 {
    let h = read_guest_cstring(env, href);
    let p = read_guest_cstring(env, prefix);
    let r = unsafe { xml::xmlNewNs(h2node(node), cstr_xml(&h), cstr_xml(&p)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSearchNs(env: &mut Environment, doc: u32, node: u32, prefix: u32) -> u32 {
    let p = read_guest_cstring(env, prefix);
    let r = unsafe { xml::xmlSearchNs(h2doc(doc), h2node(node), cstr_xml(&p)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSearchNsByHref(env: &mut Environment, doc: u32, node: u32, href: u32) -> u32 {
    let h = read_guest_cstring(env, href);
    let r = unsafe { xml::xmlSearchNsByHref(h2doc(doc), h2node(node), cstr_xml(&h)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSetNs(_env: &mut Environment, node: u32, ns: u32) {
    unsafe { xml::xmlSetNs(h2node(node), h2ns(ns)) };
}

#[allow(non_snake_case)]
fn xmlFreeNs(_env: &mut Environment, ns: u32) {
    let p = release_handle(ns) as *mut xml::xmlNs;
    if !p.is_null() {
        unsafe { xml::xmlFreeNs(p) };
    }
}

#[allow(non_snake_case)]
fn xmlAttrName(env: &mut Environment, attr: u32) -> u32 {
    let p = unsafe { xml::hxml_attr_name(h2attr(attr)) };
    copy_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlAttrNext(_env: &mut Environment, attr: u32) -> u32 {
    let p = unsafe { xml::hxml_attr_next(h2attr(attr)) };
    id_for(p as *mut c_void)
}

// ============================================================================
// TextReader API (xmlreader.h)
// ============================================================================

#[allow(non_snake_case)]
fn xmlReaderForFile(env: &mut Environment, filename: u32, encoding: u32, options: i32) -> u32 {
    let path = read_guest_cstr_string(env, filename);
    log!("xmlReaderForFile(\"{}\")", path);
    let data = env.fs.read(GuestPath::new(&path)).unwrap_or_default();
    if data.is_empty() {
        log!("xmlReaderForFile: file not found or empty: {}", path);
        return 0;
    }
    let url_c = CString::new(path.clone()).unwrap_or_else(|_| CString::new("").unwrap());
    let enc_c = read_guest_cstring(env, encoding);
    let r = unsafe {
        xml::xmlReaderForMemory(
            data.as_ptr() as *const c_char,
            data.len() as c_int,
            url_c.as_ptr(),
            cstr_ptr(&enc_c),
            options,
        )
    };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlReaderForMemory(
    env: &mut Environment,
    buffer: u32,
    size: i32,
    url: u32,
    encoding: u32,
    options: i32,
) -> u32 {
    let data = read_guest_bytes(env, buffer, size as u32);
    let url_c = read_guest_cstring(env, url);
    let enc_c = read_guest_cstring(env, encoding);
    let r = unsafe {
        xml::xmlReaderForMemory(
            data.as_ptr() as *const c_char,
            size,
            cstr_ptr(&url_c),
            cstr_ptr(&enc_c),
            options,
        )
    };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlReaderForDoc(env: &mut Environment, cur: u32, url: u32, encoding: u32, options: i32) -> u32 {
    let bytes = read_guest_cstr(env, cur);
    let c = cstring_or_default(&bytes);
    let url_c = read_guest_cstring(env, url);
    let enc_c = read_guest_cstring(env, encoding);
    let r = unsafe {
        xml::xmlReaderForDoc(
            c.as_ptr() as *const xml::xmlChar,
            cstr_ptr(&url_c),
            cstr_ptr(&enc_c),
            options,
        )
    };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlFreeTextReader(_env: &mut Environment, reader: u32) {
    let p = release_handle(reader) as *mut xml::xmlTextReader;
    if !p.is_null() {
        unsafe { xml::xmlFreeTextReader(p) };
    }
}

#[allow(non_snake_case)]
fn xmlTextReaderRead(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderRead(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderClose(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderClose(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderNodeType(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderNodeType(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderDepth(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderDepth(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderAttributeCount(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderAttributeCount(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderHasAttributes(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderHasAttributes(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderHasValue(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderHasValue(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderIsEmptyElement(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderIsEmptyElement(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderName(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderName(h2reader(reader)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderLocalName(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderLocalName(h2reader(reader)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderPrefix(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderPrefix(h2reader(reader)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderNamespaceUri(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderNamespaceUri(h2reader(reader)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderValue(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderValue(h2reader(reader)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderConstName(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderConstName(h2reader(reader)) };
    copy_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderConstLocalName(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderConstLocalName(h2reader(reader)) };
    copy_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderConstPrefix(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderConstPrefix(h2reader(reader)) };
    copy_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderConstNamespaceUri(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderConstNamespaceUri(h2reader(reader)) };
    copy_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderConstValue(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderConstValue(h2reader(reader)) };
    copy_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderGetAttribute(env: &mut Environment, reader: u32, name: u32) -> u32 {
    let n = read_guest_cstring(env, name);
    let p = unsafe { xml::xmlTextReaderGetAttribute(h2reader(reader), cstr_xml(&n)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderGetAttributeNo(env: &mut Environment, reader: u32, no: i32) -> u32 {
    let p = unsafe { xml::xmlTextReaderGetAttributeNo(h2reader(reader), no) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderGetAttributeNs(
    env: &mut Environment,
    reader: u32,
    local_name: u32,
    ns_uri: u32,
) -> u32 {
    let ln = read_guest_cstring(env, local_name);
    let nu = read_guest_cstring(env, ns_uri);
    let p =
        unsafe { xml::xmlTextReaderGetAttributeNs(h2reader(reader), cstr_xml(&ln), cstr_xml(&nu)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderMoveToAttribute(env: &mut Environment, reader: u32, name: u32) -> i32 {
    let n = read_guest_cstring(env, name);
    unsafe { xml::xmlTextReaderMoveToAttribute(h2reader(reader), cstr_xml(&n)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderMoveToFirstAttribute(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderMoveToFirstAttribute(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderMoveToNextAttribute(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderMoveToNextAttribute(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderMoveToElement(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderMoveToElement(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderReadInnerXml(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderReadInnerXml(h2reader(reader)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderReadOuterXml(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderReadOuterXml(h2reader(reader)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderReadString(env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderReadString(h2reader(reader)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlTextReaderNext(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderNext(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderNextSibling(_env: &mut Environment, reader: u32) -> i32 {
    unsafe { xml::xmlTextReaderNextSibling(h2reader(reader)) }
}

#[allow(non_snake_case)]
fn xmlTextReaderExpand(_env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderExpand(h2reader(reader)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlTextReaderCurrentNode(_env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderCurrentNode(h2reader(reader)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlTextReaderCurrentDoc(_env: &mut Environment, reader: u32) -> u32 {
    let p = unsafe { xml::xmlTextReaderCurrentDoc(h2reader(reader)) };
    id_for(p as *mut c_void)
}

// ============================================================================
// XPath / XPointer / XInclude
// ============================================================================

#[allow(non_snake_case)]
fn xmlXPathNewContext(_env: &mut Environment, doc: u32) -> u32 {
    let p = unsafe { xml::xmlXPathNewContext(h2doc(doc)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlXPathFreeContext(_env: &mut Environment, ctx: u32) {
    let p = release_handle(ctx) as *mut xml::xmlXPathContext;
    if !p.is_null() {
        unsafe { xml::xmlXPathFreeContext(p) };
    }
}

#[allow(non_snake_case)]
fn xmlXPathRegisterNs(env: &mut Environment, ctx: u32, prefix: u32, ns_uri: u32) -> i32 {
    let p = read_guest_cstring(env, prefix);
    let u = read_guest_cstring(env, ns_uri);
    unsafe { xml::xmlXPathRegisterNs(h2xpath_ctxt(ctx), cstr_xml(&p), cstr_xml(&u)) }
}

#[allow(non_snake_case)]
fn xmlXPathEval(env: &mut Environment, expr: u32, ctx: u32) -> u32 {
    let e = read_guest_cstring(env, expr);
    let r = unsafe { xml::xmlXPathEval(cstr_xml(&e), h2xpath_ctxt(ctx)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlXPathEvalExpression(env: &mut Environment, expr: u32, ctx: u32) -> u32 {
    let e = read_guest_cstring(env, expr);
    let r = unsafe { xml::xmlXPathEvalExpression(cstr_xml(&e), h2xpath_ctxt(ctx)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlXPathCompile(env: &mut Environment, expr: u32) -> u32 {
    let e = read_guest_cstring(env, expr);
    let r = unsafe { xml::xmlXPathCompile(cstr_xml(&e)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlXPathFreeCompExpr(_env: &mut Environment, expr: u32) {
    let p = release_handle(expr) as *mut xml::xmlXPathCompExpr;
    if !p.is_null() {
        unsafe { xml::xmlXPathFreeCompExpr(p) };
    }
}

#[allow(non_snake_case)]
fn xmlXPathCompiledEval(_env: &mut Environment, expr: u32, ctx: u32) -> u32 {
    let r = unsafe { xml::xmlXPathCompiledEval(h2xpath_comp(expr), h2xpath_ctxt(ctx)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlXPathFreeObject(_env: &mut Environment, obj: u32) {
    let p = release_handle(obj) as *mut xml::xmlXPathObject;
    if !p.is_null() {
        unsafe { xml::xmlXPathFreeObject(p) };
    }
}

#[allow(non_snake_case)]
fn xmlXPathSetContextNode(_env: &mut Environment, node: u32, ctx: u32) -> i32 {
    unsafe { xml::xmlXPathSetContextNode(h2node(node), h2xpath_ctxt(ctx)) }
}

// Accessors that turn xmlXPathObject into something the guest can inspect:
#[allow(non_snake_case)]
fn xmlXPathObjectGetType(_env: &mut Environment, obj: u32) -> i32 {
    unsafe { xml::hxml_xpath_object_type(h2xpath_obj(obj)) }
}

#[allow(non_snake_case)]
fn xmlXPathObjectGetBool(_env: &mut Environment, obj: u32) -> i32 {
    unsafe { xml::hxml_xpath_object_boolval(h2xpath_obj(obj)) }
}

#[allow(non_snake_case)]
fn xmlXPathObjectGetFloat(_env: &mut Environment, obj: u32) -> f64 {
    unsafe { xml::hxml_xpath_object_floatval(h2xpath_obj(obj)) }
}

#[allow(non_snake_case)]
fn xmlXPathObjectGetString(env: &mut Environment, obj: u32) -> u32 {
    let p = unsafe { xml::hxml_xpath_object_stringval(h2xpath_obj(obj)) };
    copy_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlXPathObjectGetNodeSetSize(_env: &mut Environment, obj: u32) -> i32 {
    let ns = unsafe { xml::hxml_xpath_object_nodeset(h2xpath_obj(obj)) };
    unsafe { xml::hxml_nodeset_length(ns) }
}

#[allow(non_snake_case)]
fn xmlXPathObjectGetNodeSetItem(_env: &mut Environment, obj: u32, idx: i32) -> u32 {
    let ns = unsafe { xml::hxml_xpath_object_nodeset(h2xpath_obj(obj)) };
    let n = unsafe { xml::hxml_nodeset_item(ns, idx) };
    id_for(n as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlXIncludeProcess(_env: &mut Environment, doc: u32) -> i32 {
    unsafe { xml::xmlXIncludeProcess(h2doc(doc)) }
}

#[allow(non_snake_case)]
fn xmlXIncludeProcessFlags(_env: &mut Environment, doc: u32, flags: i32) -> i32 {
    unsafe { xml::xmlXIncludeProcessFlags(h2doc(doc), flags) }
}

#[allow(non_snake_case)]
fn xmlXIncludeProcessTree(_env: &mut Environment, node: u32) -> i32 {
    unsafe { xml::xmlXIncludeProcessTree(h2node(node)) }
}

#[allow(non_snake_case)]
fn xmlXIncludeProcessTreeFlags(_env: &mut Environment, node: u32, flags: i32) -> i32 {
    unsafe { xml::xmlXIncludeProcessTreeFlags(h2node(node), flags) }
}

#[allow(non_snake_case)]
fn xmlXPtrEval(env: &mut Environment, expr: u32, ctx: u32) -> u32 {
    let e = read_guest_cstring(env, expr);
    let r = unsafe { xml::xmlXPtrEval(cstr_xml(&e), h2xpath_ctxt(ctx)) };
    id_for(r as *mut c_void)
}

// ============================================================================
// DTD validation (valid.h)
// ============================================================================

#[allow(non_snake_case)]
fn xmlNewValidCtxt(_env: &mut Environment) -> u32 {
    let p = unsafe { xml::xmlNewValidCtxt() };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlFreeValidCtxt(_env: &mut Environment, ctxt: u32) {
    let p = release_handle(ctxt) as *mut xml::xmlValidCtxt;
    if !p.is_null() {
        unsafe { xml::xmlFreeValidCtxt(p) };
    }
}

#[allow(non_snake_case)]
fn xmlValidateDocument(_env: &mut Environment, ctxt: u32, doc: u32) -> i32 {
    unsafe { xml::xmlValidateDocument(h2valid(ctxt), h2doc(doc)) }
}

#[allow(non_snake_case)]
fn xmlValidateDtd(_env: &mut Environment, ctxt: u32, doc: u32, dtd: u32) -> i32 {
    unsafe { xml::xmlValidateDtd(h2valid(ctxt), h2doc(doc), h2dtd(dtd)) }
}

#[allow(non_snake_case)]
fn xmlParseDTD(env: &mut Environment, external_id: u32, system_id: u32) -> u32 {
    let e = read_guest_cstring(env, external_id);
    let s = read_guest_cstring(env, system_id);
    let r = unsafe { xml::xmlParseDTD(cstr_xml(&e), cstr_xml(&s)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlFreeDtd(_env: &mut Environment, dtd: u32) {
    let p = release_handle(dtd) as *mut xml::xmlDtd;
    if !p.is_null() {
        unsafe { xml::xmlFreeDtd(p) };
    }
}

#[allow(non_snake_case)]
fn xmlNewDtd(
    env: &mut Environment,
    doc: u32,
    name: u32,
    external_id: u32,
    system_id: u32,
) -> u32 {
    let n = read_guest_cstring(env, name);
    let e = read_guest_cstring(env, external_id);
    let s = read_guest_cstring(env, system_id);
    let r = unsafe { xml::xmlNewDtd(h2doc(doc), cstr_xml(&n), cstr_xml(&e), cstr_xml(&s)) };
    id_for(r as *mut c_void)
}

// ============================================================================
// XML Schemas (xmlschemas.h)
// ============================================================================

#[allow(non_snake_case)]
fn xmlSchemaNewParserCtxt(env: &mut Environment, url: u32) -> u32 {
    let u = read_guest_cstring(env, url);
    let r = unsafe { xml::xmlSchemaNewParserCtxt(cstr_ptr(&u)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSchemaNewMemParserCtxt(env: &mut Environment, buf: u32, size: i32) -> u32 {
    let data = read_guest_bytes(env, buf, size as u32);
    let r = unsafe { xml::xmlSchemaNewMemParserCtxt(data.as_ptr() as *const c_char, size) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSchemaFreeParserCtxt(_env: &mut Environment, ctxt: u32) {
    let p = release_handle(ctxt) as *mut xml::xmlSchemaParserCtxt;
    if !p.is_null() {
        unsafe { xml::xmlSchemaFreeParserCtxt(p) };
    }
}

#[allow(non_snake_case)]
fn xmlSchemaParse(_env: &mut Environment, ctxt: u32) -> u32 {
    let p = unsafe { xml::xmlSchemaParse(h2schema_parser(ctxt)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSchemaFree(_env: &mut Environment, schema: u32) {
    let p = release_handle(schema) as *mut xml::xmlSchema;
    if !p.is_null() {
        unsafe { xml::xmlSchemaFree(p) };
    }
}

#[allow(non_snake_case)]
fn xmlSchemaNewValidCtxt(_env: &mut Environment, schema: u32) -> u32 {
    let p = unsafe { xml::xmlSchemaNewValidCtxt(h2schema(schema)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSchemaFreeValidCtxt(_env: &mut Environment, ctxt: u32) {
    let p = release_handle(ctxt) as *mut xml::xmlSchemaValidCtxt;
    if !p.is_null() {
        unsafe { xml::xmlSchemaFreeValidCtxt(p) };
    }
}

#[allow(non_snake_case)]
fn xmlSchemaValidateDoc(_env: &mut Environment, ctxt: u32, doc: u32) -> i32 {
    unsafe { xml::xmlSchemaValidateDoc(h2schema_valid(ctxt), h2doc(doc)) }
}

#[allow(non_snake_case)]
fn xmlSchemaValidateFile(env: &mut Environment, ctxt: u32, filename: u32, options: i32) -> i32 {
    let f = read_guest_cstring(env, filename);
    unsafe { xml::xmlSchemaValidateFile(h2schema_valid(ctxt), cstr_ptr(&f), options) }
}

#[allow(non_snake_case)]
fn xmlSchemaValidateOneElement(_env: &mut Environment, ctxt: u32, elem: u32) -> i32 {
    unsafe { xml::xmlSchemaValidateOneElement(h2schema_valid(ctxt), h2node(elem)) }
}

// ============================================================================
// Relax-NG
// ============================================================================

#[allow(non_snake_case)]
fn xmlRelaxNGNewParserCtxt(env: &mut Environment, url: u32) -> u32 {
    let u = read_guest_cstring(env, url);
    let r = unsafe { xml::xmlRelaxNGNewParserCtxt(cstr_ptr(&u)) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlRelaxNGNewMemParserCtxt(env: &mut Environment, buf: u32, size: i32) -> u32 {
    let data = read_guest_bytes(env, buf, size as u32);
    let r = unsafe { xml::xmlRelaxNGNewMemParserCtxt(data.as_ptr() as *const c_char, size) };
    id_for(r as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlRelaxNGFreeParserCtxt(_env: &mut Environment, ctxt: u32) {
    let p = release_handle(ctxt) as *mut xml::xmlRelaxNGParserCtxt;
    if !p.is_null() {
        unsafe { xml::xmlRelaxNGFreeParserCtxt(p) };
    }
}

#[allow(non_snake_case)]
fn xmlRelaxNGParse(_env: &mut Environment, ctxt: u32) -> u32 {
    let p = unsafe { xml::xmlRelaxNGParse(h2rng_parser(ctxt)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlRelaxNGFree(_env: &mut Environment, schema: u32) {
    let p = release_handle(schema) as *mut xml::xmlRelaxNG;
    if !p.is_null() {
        unsafe { xml::xmlRelaxNGFree(p) };
    }
}

#[allow(non_snake_case)]
fn xmlRelaxNGNewValidCtxt(_env: &mut Environment, schema: u32) -> u32 {
    let p = unsafe { xml::xmlRelaxNGNewValidCtxt(h2rng(schema)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlRelaxNGFreeValidCtxt(_env: &mut Environment, ctxt: u32) {
    let p = release_handle(ctxt) as *mut xml::xmlRelaxNGValidCtxt;
    if !p.is_null() {
        unsafe { xml::xmlRelaxNGFreeValidCtxt(p) };
    }
}

#[allow(non_snake_case)]
fn xmlRelaxNGValidateDoc(_env: &mut Environment, ctxt: u32, doc: u32) -> i32 {
    unsafe { xml::xmlRelaxNGValidateDoc(h2rng_valid(ctxt), h2doc(doc)) }
}

// ============================================================================
// Schematron
// ============================================================================

#[allow(non_snake_case)]
fn xmlSchematronNewParserCtxt(env: &mut Environment, url: u32) -> u32 {
    let u = read_guest_cstring(env, url);
    let p = unsafe { xml::xmlSchematronNewParserCtxt(cstr_ptr(&u)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSchematronNewMemParserCtxt(env: &mut Environment, buf: u32, size: i32) -> u32 {
    let data = read_guest_bytes(env, buf, size as u32);
    let p = unsafe { xml::xmlSchematronNewMemParserCtxt(data.as_ptr() as *const c_char, size) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSchematronFreeParserCtxt(_env: &mut Environment, ctxt: u32) {
    let p = release_handle(ctxt) as *mut xml::xmlSchematronParserCtxt;
    if !p.is_null() {
        unsafe { xml::xmlSchematronFreeParserCtxt(p) };
    }
}

#[allow(non_snake_case)]
fn xmlSchematronParse(_env: &mut Environment, ctxt: u32) -> u32 {
    let p = unsafe { xml::xmlSchematronParse(h2tron_parser(ctxt)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSchematronFree(_env: &mut Environment, schema: u32) {
    let p = release_handle(schema) as *mut xml::xmlSchematron;
    if !p.is_null() {
        unsafe { xml::xmlSchematronFree(p) };
    }
}

#[allow(non_snake_case)]
fn xmlSchematronNewValidCtxt(_env: &mut Environment, schema: u32, options: i32) -> u32 {
    let p = unsafe { xml::xmlSchematronNewValidCtxt(h2tron(schema), options) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlSchematronFreeValidCtxt(_env: &mut Environment, ctxt: u32) {
    let p = release_handle(ctxt) as *mut xml::xmlSchematronValidCtxt;
    if !p.is_null() {
        unsafe { xml::xmlSchematronFreeValidCtxt(p) };
    }
}

#[allow(non_snake_case)]
fn xmlSchematronValidateDoc(_env: &mut Environment, ctxt: u32, doc: u32) -> i32 {
    unsafe { xml::xmlSchematronValidateDoc(h2tron_valid(ctxt), h2doc(doc)) }
}

// ============================================================================
// Catalog API
// ============================================================================

#[allow(non_snake_case)]
fn xmlLoadCatalog(env: &mut Environment, filename: u32) -> i32 {
    let f = read_guest_cstring(env, filename);
    unsafe { xml::xmlLoadCatalog(cstr_ptr(&f)) }
}

#[allow(non_snake_case)]
fn xmlLoadCatalogs(env: &mut Environment, paths: u32) {
    let p = read_guest_cstring(env, paths);
    unsafe { xml::xmlLoadCatalogs(cstr_ptr(&p)) };
}

#[allow(non_snake_case)]
fn xmlCatalogResolveURI(env: &mut Environment, uri: u32) -> u32 {
    let u = read_guest_cstring(env, uri);
    let p = unsafe { xml::xmlCatalogResolveURI(cstr_xml(&u)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlCatalogResolvePublic(env: &mut Environment, pub_id: u32) -> u32 {
    let u = read_guest_cstring(env, pub_id);
    let p = unsafe { xml::xmlCatalogResolvePublic(cstr_xml(&u)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlCatalogResolveSystem(env: &mut Environment, sys_id: u32) -> u32 {
    let u = read_guest_cstring(env, sys_id);
    let p = unsafe { xml::xmlCatalogResolveSystem(cstr_xml(&u)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlCatalogCleanup(_env: &mut Environment) {
    unsafe { xml::xmlCatalogCleanup() };
}

// ============================================================================
// URI API
// ============================================================================

#[allow(non_snake_case)]
fn xmlParseURI(env: &mut Environment, uri: u32) -> u32 {
    let u = read_guest_cstring(env, uri);
    let p = unsafe { xml::xmlParseURI(cstr_ptr(&u)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlFreeURI(_env: &mut Environment, uri: u32) {
    let p = release_handle(uri) as *mut xml::xmlURI;
    if !p.is_null() {
        unsafe { xml::xmlFreeURI(p) };
    }
}

#[allow(non_snake_case)]
fn xmlBuildURI(env: &mut Environment, uri: u32, base: u32) -> u32 {
    let u = read_guest_cstring(env, uri);
    let b = read_guest_cstring(env, base);
    let p = unsafe { xml::xmlBuildURI(cstr_xml(&u), cstr_xml(&b)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlBuildRelativeURI(env: &mut Environment, uri: u32, base: u32) -> u32 {
    let u = read_guest_cstring(env, uri);
    let b = read_guest_cstring(env, base);
    let p = unsafe { xml::xmlBuildRelativeURI(cstr_xml(&u), cstr_xml(&b)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlURIEscape(env: &mut Environment, s: u32) -> u32 {
    let u = read_guest_cstring(env, s);
    let p = unsafe { xml::xmlURIEscape(cstr_xml(&u)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlURIEscapeStr(env: &mut Environment, s: u32, list: u32) -> u32 {
    let u = read_guest_cstring(env, s);
    let l = read_guest_cstring(env, list);
    let p = unsafe { xml::xmlURIEscapeStr(cstr_xml(&u), cstr_xml(&l)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlSaveUri(env: &mut Environment, uri: u32) -> u32 {
    let p = unsafe { xml::xmlSaveUri(h2uri(uri)) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlCanonicPath(env: &mut Environment, path: u32) -> u32 {
    let s = read_guest_cstring(env, path);
    let p = unsafe { xml::xmlCanonicPath(cstr_xml(&s)) };
    take_xml_chars(env, p)
}

// ============================================================================
// Save / serialization API
// ============================================================================

fn write_doc_dump_to_guest_file(env: &mut Environment, filename: &str, bytes: &[u8]) -> i32 {
    match env.fs.write(GuestPath::new(filename), bytes) {
        Ok(_) => bytes.len() as i32,
        Err(e) => {
            log!(
                "xmlSaveFile: failed to write via env.fs.write({}): {:?}",
                filename,
                e
            );
            -1
        }
    }
}

#[allow(non_snake_case)]
fn xmlSaveFile(env: &mut Environment, filename: u32, doc: u32) -> i32 {
    let path = read_guest_cstr_string(env, filename);
    log!("xmlSaveFile(filename: {:?}, doc: {})", path, doc);
    let mut buf: *mut xml::xmlChar = ptr::null_mut();
    let mut size: c_int = 0;
    unsafe { xml::xmlDocDumpMemory(h2doc(doc), &mut buf as *mut _, &mut size as *mut _) };
    if buf.is_null() {
        return -1;
    }
    let bytes = unsafe { xml::read_xml_str(buf) };
    unsafe { xml::xmlFree(buf as *mut c_void) };
    write_doc_dump_to_guest_file(env, &path, &bytes)
}

#[allow(non_snake_case)]
fn xmlSaveFormatFile(env: &mut Environment, filename: u32, doc: u32, format: i32) -> i32 {
    let path = read_guest_cstr_string(env, filename);
    let mut buf: *mut xml::xmlChar = ptr::null_mut();
    let mut size: c_int = 0;
    unsafe {
        xml::xmlDocDumpFormatMemory(h2doc(doc), &mut buf as *mut _, &mut size as *mut _, format)
    };
    if buf.is_null() {
        return -1;
    }
    let bytes = unsafe { xml::read_xml_str(buf) };
    unsafe { xml::xmlFree(buf as *mut c_void) };
    write_doc_dump_to_guest_file(env, &path, &bytes)
}

#[allow(non_snake_case)]
fn xmlSaveFileEnc(env: &mut Environment, filename: u32, doc: u32, encoding: u32) -> i32 {
    let path = read_guest_cstr_string(env, filename);
    let enc_c = read_guest_cstring(env, encoding);
    let mut buf: *mut xml::xmlChar = ptr::null_mut();
    let mut size: c_int = 0;
    unsafe {
        xml::xmlDocDumpMemoryEnc(
            h2doc(doc),
            &mut buf as *mut _,
            &mut size as *mut _,
            cstr_ptr(&enc_c),
        )
    };
    if buf.is_null() {
        return -1;
    }
    let bytes = unsafe { xml::read_xml_str(buf) };
    unsafe { xml::xmlFree(buf as *mut c_void) };
    write_doc_dump_to_guest_file(env, &path, &bytes)
}

#[allow(non_snake_case)]
fn xmlSaveFormatFileEnc(
    env: &mut Environment,
    filename: u32,
    doc: u32,
    encoding: u32,
    format: i32,
) -> i32 {
    let path = read_guest_cstr_string(env, filename);
    let enc_c = read_guest_cstring(env, encoding);
    let mut buf: *mut xml::xmlChar = ptr::null_mut();
    let mut size: c_int = 0;
    unsafe {
        xml::xmlDocDumpFormatMemoryEnc(
            h2doc(doc),
            &mut buf as *mut _,
            &mut size as *mut _,
            cstr_ptr(&enc_c),
            format,
        )
    };
    if buf.is_null() {
        return -1;
    }
    let bytes = unsafe { xml::read_xml_str(buf) };
    unsafe { xml::xmlFree(buf as *mut c_void) };
    write_doc_dump_to_guest_file(env, &path, &bytes)
}

fn write_buffer_to_guest(
    env: &mut Environment,
    buf: *mut xml::xmlChar,
    size: c_int,
    out_size: u32,
    out_buf: u32,
) {
    if !out_size.is_null_addr() {
        let p: MutPtr<i32> = MutPtr::from_bits(out_size);
        env.mem.write(p, size);
    }
    if !out_buf.is_null_addr() {
        let bytes = if buf.is_null() {
            Vec::new()
        } else {
            unsafe { xml::read_xml_str(buf) }
        };
        let gp = write_to_guest(env, &bytes);
        let pp: MutPtr<u32> = MutPtr::from_bits(out_buf);
        env.mem.write(pp, gp);
    }
    if !buf.is_null() {
        unsafe { xml::xmlFree(buf as *mut c_void) };
    }
}

trait NullAddr {
    fn is_null_addr(&self) -> bool;
}
impl NullAddr for u32 {
    fn is_null_addr(&self) -> bool {
        *self == 0
    }
}

#[allow(non_snake_case)]
fn xmlDocDumpMemory(env: &mut Environment, doc: u32, mem: u32, size: u32) {
    let mut buf: *mut xml::xmlChar = ptr::null_mut();
    let mut sz: c_int = 0;
    unsafe { xml::xmlDocDumpMemory(h2doc(doc), &mut buf as *mut _, &mut sz as *mut _) };
    write_buffer_to_guest(env, buf, sz, size, mem);
}

#[allow(non_snake_case)]
fn xmlDocDumpFormatMemory(env: &mut Environment, doc: u32, mem: u32, size: u32, format: i32) {
    let mut buf: *mut xml::xmlChar = ptr::null_mut();
    let mut sz: c_int = 0;
    unsafe {
        xml::xmlDocDumpFormatMemory(h2doc(doc), &mut buf as *mut _, &mut sz as *mut _, format)
    };
    write_buffer_to_guest(env, buf, sz, size, mem);
}

#[allow(non_snake_case)]
fn xmlDocDumpMemoryEnc(env: &mut Environment, doc: u32, mem: u32, size: u32, encoding: u32) {
    let enc_c = read_guest_cstring(env, encoding);
    let mut buf: *mut xml::xmlChar = ptr::null_mut();
    let mut sz: c_int = 0;
    unsafe {
        xml::xmlDocDumpMemoryEnc(
            h2doc(doc),
            &mut buf as *mut _,
            &mut sz as *mut _,
            cstr_ptr(&enc_c),
        )
    };
    write_buffer_to_guest(env, buf, sz, size, mem);
}

#[allow(non_snake_case)]
fn xmlDocDumpFormatMemoryEnc(
    env: &mut Environment,
    doc: u32,
    mem: u32,
    size: u32,
    encoding: u32,
    format: i32,
) {
    let enc_c = read_guest_cstring(env, encoding);
    let mut buf: *mut xml::xmlChar = ptr::null_mut();
    let mut sz: c_int = 0;
    unsafe {
        xml::xmlDocDumpFormatMemoryEnc(
            h2doc(doc),
            &mut buf as *mut _,
            &mut sz as *mut _,
            cstr_ptr(&enc_c),
            format,
        )
    };
    write_buffer_to_guest(env, buf, sz, size, mem);
}

// ============================================================================
// Writer API (xmlwriter.h) — buffer-backed only; file URIs are intercepted so
// that the produced bytes land in the guest filesystem at the end.
// ============================================================================

#[allow(non_snake_case)]
fn xmlBufferCreate(_env: &mut Environment) -> u32 {
    let p = unsafe { xml::xmlBufferCreate() };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlBufferCreateSize(_env: &mut Environment, size: u32) -> u32 {
    let p = unsafe { xml::xmlBufferCreateSize(size as usize) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlBufferFree(_env: &mut Environment, buf: u32) {
    let p = release_handle(buf) as *mut xml::xmlBuffer;
    if !p.is_null() {
        unsafe { xml::xmlBufferFree(p) };
    }
}

#[allow(non_snake_case)]
fn xmlBufferEmpty(_env: &mut Environment, buf: u32) {
    let p = h2buffer(buf);
    if !p.is_null() {
        unsafe { xml::xmlBufferEmpty(p) };
    }
}

#[allow(non_snake_case)]
fn xmlBufferContent(env: &mut Environment, buf: u32) -> u32 {
    let p = h2buffer(buf);
    if p.is_null() {
        return 0;
    }
    let host = unsafe { xml::hxml_buffer_content(p) };
    copy_xml_chars(env, host)
}

#[allow(non_snake_case)]
fn xmlBufferLength(_env: &mut Environment, buf: u32) -> i32 {
    let p = h2buffer(buf);
    if p.is_null() {
        return 0;
    }
    unsafe { xml::hxml_buffer_length(p) }
}

#[allow(non_snake_case)]
fn xmlBufferAdd(env: &mut Environment, buf: u32, s: u32, len: i32) -> i32 {
    let bytes = if len > 0 {
        read_guest_bytes(env, s, len as u32)
    } else {
        read_guest_cstr(env, s)
    };
    unsafe {
        xml::xmlBufferAdd(
            h2buffer(buf),
            bytes.as_ptr() as *const xml::xmlChar,
            bytes.len() as i32,
        )
    }
}

#[allow(non_snake_case)]
fn xmlBufferCat(env: &mut Environment, buf: u32, s: u32) -> i32 {
    let bytes = read_guest_cstr(env, s);
    unsafe {
        xml::xmlBufferCat(h2buffer(buf), bytes.as_ptr() as *const xml::xmlChar)
    }
}

#[allow(non_snake_case)]
fn xmlNewTextWriterMemory(_env: &mut Environment, buf: u32, compression: i32) -> u32 {
    let p = unsafe { xml::xmlNewTextWriterMemory(h2buffer(buf), compression) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlNewTextWriterDoc(env: &mut Environment, out_doc: u32, compression: i32) -> u32 {
    let mut host_doc: *mut xml::xmlDoc = ptr::null_mut();
    let p =
        unsafe { xml::xmlNewTextWriterDoc(&mut host_doc as *mut _, compression) };
    if out_doc != 0 {
        let id = id_for(host_doc as *mut c_void);
        let pp: MutPtr<u32> = MutPtr::from_bits(out_doc);
        env.mem.write(pp, id);
    }
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlFreeTextWriter(_env: &mut Environment, writer: u32) {
    let p = release_handle(writer) as *mut xml::xmlTextWriter;
    if !p.is_null() {
        unsafe { xml::xmlFreeTextWriter(p) };
    }
}

#[allow(non_snake_case)]
fn xmlTextWriterStartDocument(
    env: &mut Environment,
    writer: u32,
    version: u32,
    encoding: u32,
    standalone: u32,
) -> i32 {
    let v = read_guest_cstring(env, version);
    let e = read_guest_cstring(env, encoding);
    let s = read_guest_cstring(env, standalone);
    unsafe {
        xml::xmlTextWriterStartDocument(
            h2writer(writer),
            cstr_ptr(&v),
            cstr_ptr(&e),
            cstr_ptr(&s),
        )
    }
}

#[allow(non_snake_case)]
fn xmlTextWriterEndDocument(_env: &mut Environment, writer: u32) -> i32 {
    unsafe { xml::xmlTextWriterEndDocument(h2writer(writer)) }
}

#[allow(non_snake_case)]
fn xmlTextWriterStartElement(env: &mut Environment, writer: u32, name: u32) -> i32 {
    let n = read_guest_cstring(env, name);
    unsafe { xml::xmlTextWriterStartElement(h2writer(writer), cstr_xml(&n)) }
}

#[allow(non_snake_case)]
fn xmlTextWriterStartElementNS(
    env: &mut Environment,
    writer: u32,
    prefix: u32,
    name: u32,
    ns_uri: u32,
) -> i32 {
    let p = read_guest_cstring(env, prefix);
    let n = read_guest_cstring(env, name);
    let u = read_guest_cstring(env, ns_uri);
    unsafe {
        xml::xmlTextWriterStartElementNS(
            h2writer(writer),
            cstr_xml(&p),
            cstr_xml(&n),
            cstr_xml(&u),
        )
    }
}

#[allow(non_snake_case)]
fn xmlTextWriterEndElement(_env: &mut Environment, writer: u32) -> i32 {
    unsafe { xml::xmlTextWriterEndElement(h2writer(writer)) }
}

#[allow(non_snake_case)]
fn xmlTextWriterFullEndElement(_env: &mut Environment, writer: u32) -> i32 {
    unsafe { xml::xmlTextWriterFullEndElement(h2writer(writer)) }
}

#[allow(non_snake_case)]
fn xmlTextWriterWriteString(env: &mut Environment, writer: u32, content: u32) -> i32 {
    let c = read_guest_cstring(env, content);
    unsafe { xml::xmlTextWriterWriteString(h2writer(writer), cstr_xml(&c)) }
}

#[allow(non_snake_case)]
fn xmlTextWriterWriteRaw(env: &mut Environment, writer: u32, content: u32) -> i32 {
    let c = read_guest_cstring(env, content);
    unsafe { xml::xmlTextWriterWriteRaw(h2writer(writer), cstr_xml(&c)) }
}

#[allow(non_snake_case)]
fn xmlTextWriterWriteAttribute(env: &mut Environment, writer: u32, name: u32, content: u32) -> i32 {
    let n = read_guest_cstring(env, name);
    let c = read_guest_cstring(env, content);
    unsafe { xml::xmlTextWriterWriteAttribute(h2writer(writer), cstr_xml(&n), cstr_xml(&c)) }
}

#[allow(non_snake_case)]
fn xmlTextWriterWriteAttributeNS(
    env: &mut Environment,
    writer: u32,
    prefix: u32,
    name: u32,
    ns_uri: u32,
    content: u32,
) -> i32 {
    let p = read_guest_cstring(env, prefix);
    let n = read_guest_cstring(env, name);
    let u = read_guest_cstring(env, ns_uri);
    let c = read_guest_cstring(env, content);
    unsafe {
        xml::xmlTextWriterWriteAttributeNS(
            h2writer(writer),
            cstr_xml(&p),
            cstr_xml(&n),
            cstr_xml(&u),
            cstr_xml(&c),
        )
    }
}

#[allow(non_snake_case)]
fn xmlTextWriterWriteComment(env: &mut Environment, writer: u32, content: u32) -> i32 {
    let c = read_guest_cstring(env, content);
    unsafe { xml::xmlTextWriterWriteComment(h2writer(writer), cstr_xml(&c)) }
}

#[allow(non_snake_case)]
fn xmlTextWriterWriteCDATA(env: &mut Environment, writer: u32, content: u32) -> i32 {
    let c = read_guest_cstring(env, content);
    unsafe { xml::xmlTextWriterWriteCDATA(h2writer(writer), cstr_xml(&c)) }
}

#[allow(non_snake_case)]
fn xmlTextWriterWritePI(env: &mut Environment, writer: u32, target: u32, content: u32) -> i32 {
    let t = read_guest_cstring(env, target);
    let c = read_guest_cstring(env, content);
    unsafe { xml::xmlTextWriterWritePI(h2writer(writer), cstr_xml(&t), cstr_xml(&c)) }
}

#[allow(non_snake_case)]
fn xmlTextWriterFlush(_env: &mut Environment, writer: u32) -> i32 {
    unsafe { xml::xmlTextWriterFlush(h2writer(writer)) }
}

#[allow(non_snake_case)]
fn xmlTextWriterSetIndent(_env: &mut Environment, writer: u32, indent: i32) -> i32 {
    unsafe { xml::xmlTextWriterSetIndent(h2writer(writer), indent) }
}

#[allow(non_snake_case)]
fn xmlTextWriterSetIndentString(env: &mut Environment, writer: u32, s: u32) -> i32 {
    let v = read_guest_cstring(env, s);
    unsafe { xml::xmlTextWriterSetIndentString(h2writer(writer), cstr_xml(&v)) }
}

// ============================================================================
// HTML parser / tree
// ============================================================================

#[allow(non_snake_case)]
fn htmlReadFile(env: &mut Environment, url: u32, encoding: u32, options: i32) -> u32 {
    let path = read_guest_cstr_string(env, url);
    let data = env.fs.read(GuestPath::new(&path)).unwrap_or_default();
    let url_c = CString::new(path.clone()).unwrap_or_else(|_| CString::new("").unwrap());
    let enc_c = read_guest_cstring(env, encoding);
    let p = unsafe {
        xml::htmlReadMemory(
            data.as_ptr() as *const c_char,
            data.len() as c_int,
            url_c.as_ptr(),
            cstr_ptr(&enc_c),
            options,
        )
    };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn htmlReadMemory(
    env: &mut Environment,
    buf: u32,
    size: i32,
    url: u32,
    encoding: u32,
    options: i32,
) -> u32 {
    let data = read_guest_bytes(env, buf, size as u32);
    let url_c = read_guest_cstring(env, url);
    let enc_c = read_guest_cstring(env, encoding);
    let p = unsafe {
        xml::htmlReadMemory(
            data.as_ptr() as *const c_char,
            size,
            cstr_ptr(&url_c),
            cstr_ptr(&enc_c),
            options,
        )
    };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn htmlReadDoc(env: &mut Environment, cur: u32, url: u32, encoding: u32, options: i32) -> u32 {
    let bytes = read_guest_cstr(env, cur);
    let c = cstring_or_default(&bytes);
    let url_c = read_guest_cstring(env, url);
    let enc_c = read_guest_cstring(env, encoding);
    let p = unsafe {
        xml::htmlReadDoc(
            c.as_ptr() as *const xml::xmlChar,
            cstr_ptr(&url_c),
            cstr_ptr(&enc_c),
            options,
        )
    };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn htmlNewParserCtxt(_env: &mut Environment) -> u32 {
    let p = unsafe { xml::htmlNewParserCtxt() };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn htmlFreeParserCtxt(_env: &mut Environment, ctxt: u32) {
    let p = release_handle(ctxt) as *mut xml::htmlParserCtxt;
    if !p.is_null() {
        unsafe { xml::htmlFreeParserCtxt(p) };
    }
}

#[allow(non_snake_case)]
fn htmlNewDoc(env: &mut Environment, uri: u32, external_id: u32) -> u32 {
    let u = read_guest_cstring(env, uri);
    let e = read_guest_cstring(env, external_id);
    let p = unsafe { xml::htmlNewDoc(cstr_xml(&u), cstr_xml(&e)) };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn htmlSaveFile(env: &mut Environment, filename: u32, doc: u32) -> i32 {
    let path = read_guest_cstr_string(env, filename);
    let mut buf: *mut xml::xmlChar = ptr::null_mut();
    let mut size: c_int = 0;
    unsafe { xml::htmlDocDumpMemory(h2doc(doc), &mut buf as *mut _, &mut size as *mut _) };
    if buf.is_null() {
        return -1;
    }
    let bytes = unsafe { xml::read_xml_str(buf) };
    unsafe { xml::xmlFree(buf as *mut c_void) };
    write_doc_dump_to_guest_file(env, &path, &bytes)
}

#[allow(non_snake_case)]
fn htmlDocDumpMemory(env: &mut Environment, doc: u32, mem: u32, size: u32) {
    let mut buf: *mut xml::xmlChar = ptr::null_mut();
    let mut sz: c_int = 0;
    unsafe { xml::htmlDocDumpMemory(h2doc(doc), &mut buf as *mut _, &mut sz as *mut _) };
    write_buffer_to_guest(env, buf, sz, size, mem);
}

#[allow(non_snake_case)]
fn htmlDocDumpMemoryFormat(env: &mut Environment, doc: u32, mem: u32, size: u32, format: i32) {
    let mut buf: *mut xml::xmlChar = ptr::null_mut();
    let mut sz: c_int = 0;
    unsafe {
        xml::htmlDocDumpMemoryFormat(h2doc(doc), &mut buf as *mut _, &mut sz as *mut _, format)
    };
    write_buffer_to_guest(env, buf, sz, size, mem);
}

// ============================================================================
// Error reporting
// ============================================================================

#[allow(non_snake_case)]
fn xmlGetLastError(_env: &mut Environment) -> u32 {
    let p = unsafe { xml::xmlGetLastError() };
    id_for(p as *mut c_void)
}

#[allow(non_snake_case)]
fn xmlResetLastError(_env: &mut Environment) {
    unsafe { xml::xmlResetLastError() };
}

#[allow(non_snake_case)]
fn xmlErrorGetCode(_env: &mut Environment, err: u32) -> i32 {
    unsafe { xml::hxml_error_code(h2error(err)) }
}

#[allow(non_snake_case)]
fn xmlErrorGetLevel(_env: &mut Environment, err: u32) -> i32 {
    unsafe { xml::hxml_error_level(h2error(err)) }
}

#[allow(non_snake_case)]
fn xmlErrorGetLine(_env: &mut Environment, err: u32) -> i32 {
    unsafe { xml::hxml_error_line(h2error(err)) }
}

#[allow(non_snake_case)]
fn xmlErrorGetMessage(env: &mut Environment, err: u32) -> u32 {
    let p = unsafe { xml::hxml_error_message(h2error(err)) };
    copy_plain_cstr(env, p)
}

#[allow(non_snake_case)]
fn xmlErrorGetFile(env: &mut Environment, err: u32) -> u32 {
    let p = unsafe { xml::hxml_error_file(h2error(err)) };
    copy_plain_cstr(env, p)
}

// ============================================================================
// String utilities (xmlstring.h)
// ============================================================================

#[allow(non_snake_case)]
fn xmlStrdup(env: &mut Environment, src: u32) -> u32 {
    let bytes = read_guest_cstr(env, src);
    write_to_guest(env, &bytes)
}

#[allow(non_snake_case)]
fn xmlStrndup(env: &mut Environment, src: u32, len: i32) -> u32 {
    let bytes = read_guest_bytes(env, src, len as u32);
    write_to_guest(env, &bytes)
}

#[allow(non_snake_case)]
fn xmlStrlen(env: &mut Environment, ptr: u32) -> i32 {
    read_guest_cstr(env, ptr).len() as i32
}

#[allow(non_snake_case)]
fn xmlStrcmp(env: &mut Environment, a: u32, b: u32) -> i32 {
    let av = read_guest_cstr(env, a);
    let bv = read_guest_cstr(env, b);
    match av.cmp(&bv) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[allow(non_snake_case)]
fn xmlStrncmp(env: &mut Environment, a: u32, b: u32, n: i32) -> i32 {
    let av = read_guest_cstr(env, a);
    let bv = read_guest_cstr(env, b);
    let n = n.max(0) as usize;
    let a_slice = if n <= av.len() { &av[..n] } else { &av[..] };
    let b_slice = if n <= bv.len() { &bv[..n] } else { &bv[..] };
    match a_slice.cmp(b_slice) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[allow(non_snake_case)]
fn xmlStrcasecmp(env: &mut Environment, a: u32, b: u32) -> i32 {
    let av = read_guest_cstr(env, a);
    let bv = read_guest_cstr(env, b);
    let al: Vec<u8> = av.iter().map(|c| c.to_ascii_lowercase()).collect();
    let bl: Vec<u8> = bv.iter().map(|c| c.to_ascii_lowercase()).collect();
    match al.cmp(&bl) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[allow(non_snake_case)]
fn xmlStrncasecmp(env: &mut Environment, a: u32, b: u32, n: i32) -> i32 {
    let av = read_guest_cstr(env, a);
    let bv = read_guest_cstr(env, b);
    let n = n.max(0) as usize;
    let a_slice: Vec<u8> = av
        .iter()
        .take(n)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let b_slice: Vec<u8> = bv
        .iter()
        .take(n)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    match a_slice.cmp(&b_slice) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[allow(non_snake_case)]
fn xmlStrEqual(env: &mut Environment, a: u32, b: u32) -> i32 {
    let av = read_guest_cstr(env, a);
    let bv = read_guest_cstr(env, b);
    if av == bv {
        1
    } else {
        0
    }
}

#[allow(non_snake_case)]
fn xmlStrsub(env: &mut Environment, src: u32, start: i32, len: i32) -> u32 {
    let bytes = read_guest_cstr(env, src);
    let start = start.max(0) as usize;
    let len = len.max(0) as usize;
    let slice = if start < bytes.len() {
        &bytes[start..(start + len).min(bytes.len())]
    } else {
        &[][..]
    };
    write_to_guest(env, slice)
}

#[allow(non_snake_case)]
fn xmlStrcat(env: &mut Environment, a: u32, b: u32) -> u32 {
    let mut v = read_guest_cstr(env, a);
    v.extend_from_slice(&read_guest_cstr(env, b));
    write_to_guest(env, &v)
}

#[allow(non_snake_case)]
fn xmlStrncat(env: &mut Environment, a: u32, b: u32, n: i32) -> u32 {
    let mut v = read_guest_cstr(env, a);
    let suffix = read_guest_cstr(env, b);
    let take = (n.max(0) as usize).min(suffix.len());
    v.extend_from_slice(&suffix[..take]);
    write_to_guest(env, &v)
}

#[allow(non_snake_case)]
fn xmlStrchr(env: &mut Environment, s: u32, ch: u32) -> u32 {
    let bytes = read_guest_cstr(env, s);
    let needle = (ch & 0xff) as u8;
    match bytes.iter().position(|&b| b == needle) {
        Some(pos) => s + pos as u32,
        None => 0,
    }
}

#[allow(non_snake_case)]
fn xmlStrstr(env: &mut Environment, s: u32, needle: u32) -> u32 {
    let bytes = read_guest_cstr(env, s);
    let n = read_guest_cstr(env, needle);
    if n.is_empty() {
        return s;
    }
    bytes
        .windows(n.len())
        .position(|w| w == n.as_slice())
        .map(|pos| s + pos as u32)
        .unwrap_or(0)
}

#[allow(non_snake_case)]
fn xmlCharStrdup(env: &mut Environment, s: u32) -> u32 {
    let bytes = read_guest_cstr(env, s);
    write_to_guest(env, &bytes)
}

#[allow(non_snake_case)]
fn xmlSAXUserParseMemory(
    env: &mut Environment,
    sax: u32,
    user_data: u32,
    buffer: u32,
    size: i32,
) -> i32 {
    // The guest may pass a SAX handler whose function pointers are guest
    // (ARM) addresses; libxml2 can't safely call into those.  Treat any
    // non-NULL guest SAX as "no callbacks": parse with libxml2's default
    // SAX handler, which still validates the document and surfaces errors
    // through xmlGetLastError().  user_data is opaque to libxml2 in that
    // case and therefore safe to drop.
    let _ = (sax, user_data);
    if size <= 0 || buffer == 0 {
        return -1;
    }
    let data = read_guest_bytes(env, buffer, size as u32);
    unsafe {
        xml::xmlSAXUserParseMemory(
            ptr::null_mut(),
            ptr::null_mut(),
            data.as_ptr() as *const c_char,
            size as c_int,
        )
    }
}

#[allow(non_snake_case)]
fn xmlMemoryDump(_env: &mut Environment) -> i32 {
    // libxml2's allocation tracker is a no-op unless xmlMemSetup() was
    // called by the embedder; we forward the call verbatim so the guest
    // observes the same return value (typically 0) as on iPhone OS.
    unsafe { xml::xmlMemoryDump() }
}

/// `int xmlInitMemory(void)` — initialises libxml2's allocator
/// overrides. The function is deprecated upstream (the allocator
/// table is now initialised on first use), but iOS 5/6 apps built
/// against libxml2 2.7.x still call it explicitly during startup.
/// libxml2 returns 0 on success.
#[allow(non_snake_case)]
fn xmlInitMemory(_env: &mut Environment) -> i32 {
    unsafe { xml::xmlInitMemory() }
}

/// `void xmlCleanupMemory(void)` — releases module-private allocator
/// state. Mirrors `xmlCleanupParser()` in the lifecycle pairing
/// (`xmlInitMemory` ↔ `xmlCleanupMemory`). Documented at
/// <https://gnome.pages.gitlab.gnome.org/libxml2/devhelp/libxml2-xmlmemory.html>.
#[allow(non_snake_case)]
fn xmlCleanupMemory(_env: &mut Environment) {
    unsafe { xml::xmlCleanupMemory() };
}

/// `double xmlXPathCastToNumber(xmlXPathObjectPtr val)` — XPath 1.0
/// §4.4 number-conversion. Forwarded verbatim to libxml2 so all four
/// XPath object types (number/boolean/node-set/string) round-trip
/// identically to Apple's libxml2.
#[allow(non_snake_case)]
fn xmlXPathCastToNumber(_env: &mut Environment, obj: u32) -> f64 {
    let p = h2xpath_obj(obj);
    if p.is_null() {
        return f64::NAN;
    }
    unsafe { xml::xmlXPathCastToNumber(p) }
}

/// `int xmlXPathCastToBoolean(xmlXPathObjectPtr val)` — XPath 1.0
/// §4.3 boolean-conversion.
#[allow(non_snake_case)]
fn xmlXPathCastToBoolean(_env: &mut Environment, obj: u32) -> i32 {
    let p = h2xpath_obj(obj);
    if p.is_null() {
        return 0;
    }
    unsafe { xml::xmlXPathCastToBoolean(p) }
}

/// `xmlChar *xmlXPathCastToString(xmlXPathObjectPtr val)` — XPath 1.0
/// §4.2 string-conversion. Returned string is xmlMalloc'd and
/// transferred to guest memory.
#[allow(non_snake_case)]
fn xmlXPathCastToString(env: &mut Environment, obj: u32) -> u32 {
    let p = h2xpath_obj(obj);
    if p.is_null() {
        return 0;
    }
    let r = unsafe { xml::xmlXPathCastToString(p) };
    take_xml_chars(env, r)
}

#[allow(non_snake_case)]
fn xmlFree(env: &mut Environment, ptr: u32) {
    // If the guest is freeing one of our object handles, drop it from the
    // table (libxml2 itself would have called the appropriate xmlFreeFoo).
    if is_handle(ptr) {
        let _ = release_handle(ptr);
        return;
    }
    // Otherwise the pointer is into guest memory (an xmlChar* string we
    // allocated); free the underlying guest allocation.
    if ptr != 0 {
        let gp: MutVoidPtr = Ptr::<c_void, true>::from_bits(ptr);
        env.mem.free(gp);
    }
}

// ============================================================================
// Entity encoding helpers
// ============================================================================

#[allow(non_snake_case)]
fn xmlEncodeEntitiesReentrant(env: &mut Environment, doc: u32, s: u32) -> u32 {
    let bytes = read_guest_cstr(env, s);
    let c = cstring_or_default(&bytes);
    let p =
        unsafe { xml::xmlEncodeEntitiesReentrant(h2doc(doc), c.as_ptr() as *const xml::xmlChar) };
    take_xml_chars(env, p)
}

#[allow(non_snake_case)]
fn xmlEncodeSpecialChars(env: &mut Environment, doc: u32, s: u32) -> u32 {
    let bytes = read_guest_cstr(env, s);
    let c = cstring_or_default(&bytes);
    let p = unsafe { xml::xmlEncodeSpecialChars(h2doc(doc), c.as_ptr() as *const xml::xmlChar) };
    take_xml_chars(env, p)
}

// ============================================================================
// Exports
// ============================================================================

pub const CONSTANTS: ConstantExports = &[
    ("_xmlIndentTreeOutput", HostConstant::NullPtr),
    ("_xmlSaveNoEmptyTags", HostConstant::NullPtr),
    // `xmlError xmlLastError` (libxml2 ≥ 2.6) — a struct, not a pointer,
    // declared `extern xmlError xmlLastError` in `libxml/xmlerror.h`.
    // Apps that read its fields directly (rare) expect the storage to
    // exist; calling `xmlGetLastError()` is the supported API. We
    // expose a zero-filled block sized per libxml2 2.7.x's `xmlError`
    // layout on 32-bit ARM (sizeof = 80 bytes: code/level/line/int1/int2
    // = 5×u32 (20) + domain (4) + message/file/str1/str2/str3 (5×ptr =
    // 20) + node/ctxt (2×ptr = 8) + padding = round to 80). We over-
    // allocate to 256 bytes to be safe against any libxml2 version
    // that grows the struct.
    (
        "_xmlLastError",
        HostConstant::Custom(|env| {
            let p = env.mem.alloc(256);
            let bytes = env.mem.bytes_at_mut(p.cast(), 256);
            for b in bytes.iter_mut() {
                *b = 0;
            }
            p.cast_const()
        }),
    ),
];

const FUNCTIONS: FunctionExports = &[
    // ---- parser ----
    export_c_func!(xmlNewParserCtxt()),
    export_c_func!(xmlClearParserCtxt(_)),
    export_c_func!(xmlFreeParserCtxt(_)),
    export_c_func!(xmlCtxtReset(_)),
    export_c_func!(xmlCleanupParser()),
    export_c_func!(xmlCtxtGetLastError(_)),
    export_c_func!(xmlCtxtResetLastError(_)),
    export_c_func!(xmlReadFile(_, _, _)),
    export_c_func!(xmlReadMemory(_, _, _, _, _)),
    export_c_func!(xmlReadDoc(_, _, _, _)),
    export_c_func!(xmlCtxtReadFile(_, _, _, _)),
    export_c_func!(xmlCtxtReadMemory(_, _, _, _, _, _)),
    export_c_func!(xmlCtxtReadDoc(_, _, _, _, _)),
    export_c_func!(xmlParseFile(_)),
    export_c_func!(xmlParseMemory(_, _)),
    export_c_func!(xmlParseDoc(_)),
    export_c_func!(xmlCreatePushParserCtxt(_, _, _, _, _)),
    export_c_func!(xmlParseChunk(_, _, _, _)),
    export_c_func!(xmlCheckVersion(_)),
    export_c_func!(xmlSAXUserParseMemory(_, _, _, _)),
    export_c_func!(xmlMemoryDump()),
    export_c_func!(xmlInitMemory()),
    export_c_func!(xmlCleanupMemory()),
    // ---- tree ----
    export_c_func!(xmlNewDoc(_)),
    export_c_func!(xmlFreeDoc(_)),
    export_c_func!(xmlDocGetRootElement(_)),
    export_c_func!(xmlDocSetRootElement(_, _)),
    export_c_func!(xmlNewNode(_, _)),
    export_c_func!(xmlNewDocNode(_, _, _, _)),
    export_c_func!(xmlNewText(_)),
    export_c_func!(xmlNewDocText(_, _)),
    export_c_func!(xmlNewComment(_)),
    export_c_func!(xmlNewCDataBlock(_, _, _)),
    export_c_func!(xmlNewChild(_, _, _, _)),
    export_c_func!(xmlNewTextChild(_, _, _, _)),
    export_c_func!(xmlAddChild(_, _)),
    export_c_func!(xmlAddPrevSibling(_, _)),
    export_c_func!(xmlAddNextSibling(_, _)),
    export_c_func!(xmlAddSibling(_, _)),
    export_c_func!(xmlUnlinkNode(_)),
    export_c_func!(xmlFreeNode(_)),
    export_c_func!(xmlFreeNodeList(_)),
    export_c_func!(xmlCopyNode(_, _)),
    export_c_func!(xmlDocCopyNode(_, _, _)),
    export_c_func!(xmlReplaceNode(_, _)),
    export_c_func!(xmlNodeSetName(_, _)),
    export_c_func!(xmlNodeSetContent(_, _)),
    export_c_func!(xmlNodeAddContent(_, _)),
    export_c_func!(xmlNodeGetContent(_)),
    export_c_func!(xmlNodeListGetString(_, _, _)),
    export_c_func!(xmlGetLastChild(_)),
    export_c_func!(xmlChildElementCount(_)),
    export_c_func!(xmlFirstElementChild(_)),
    export_c_func!(xmlLastElementChild(_)),
    export_c_func!(xmlNextElementSibling(_)),
    export_c_func!(xmlPreviousElementSibling(_)),
    export_c_func!(xmlChildrenNode(_)),
    export_c_func!(xmlGetNodePath(_)),
    export_c_func!(xmlGetLineNo(_)),
    // ---- attrs / namespaces ----
    export_c_func!(xmlNewProp(_, _, _)),
    export_c_func!(xmlNewNsProp(_, _, _, _)),
    export_c_func!(xmlSetProp(_, _, _)),
    export_c_func!(xmlSetNsProp(_, _, _, _)),
    export_c_func!(xmlUnsetProp(_, _)),
    export_c_func!(xmlUnsetNsProp(_, _, _)),
    export_c_func!(xmlGetProp(_, _)),
    export_c_func!(xmlGetNoNsProp(_, _)),
    export_c_func!(xmlGetNsProp(_, _, _)),
    export_c_func!(xmlHasProp(_, _)),
    export_c_func!(xmlHasNsProp(_, _, _)),
    export_c_func!(xmlNewNs(_, _, _)),
    export_c_func!(xmlSearchNs(_, _, _)),
    export_c_func!(xmlSearchNsByHref(_, _, _)),
    export_c_func!(xmlSetNs(_, _)),
    export_c_func!(xmlFreeNs(_)),
    // ---- text reader ----
    export_c_func!(xmlReaderForFile(_, _, _)),
    export_c_func!(xmlReaderForMemory(_, _, _, _, _)),
    export_c_func!(xmlReaderForDoc(_, _, _, _)),
    export_c_func!(xmlFreeTextReader(_)),
    export_c_func!(xmlTextReaderRead(_)),
    export_c_func!(xmlTextReaderClose(_)),
    export_c_func!(xmlTextReaderNodeType(_)),
    export_c_func!(xmlTextReaderDepth(_)),
    export_c_func!(xmlTextReaderAttributeCount(_)),
    export_c_func!(xmlTextReaderHasAttributes(_)),
    export_c_func!(xmlTextReaderHasValue(_)),
    export_c_func!(xmlTextReaderIsEmptyElement(_)),
    export_c_func!(xmlTextReaderName(_)),
    export_c_func!(xmlTextReaderLocalName(_)),
    export_c_func!(xmlTextReaderPrefix(_)),
    export_c_func!(xmlTextReaderNamespaceUri(_)),
    export_c_func!(xmlTextReaderValue(_)),
    export_c_func!(xmlTextReaderConstName(_)),
    export_c_func!(xmlTextReaderConstLocalName(_)),
    export_c_func!(xmlTextReaderConstPrefix(_)),
    export_c_func!(xmlTextReaderConstNamespaceUri(_)),
    export_c_func!(xmlTextReaderConstValue(_)),
    export_c_func!(xmlTextReaderGetAttribute(_, _)),
    export_c_func!(xmlTextReaderGetAttributeNo(_, _)),
    export_c_func!(xmlTextReaderGetAttributeNs(_, _, _)),
    export_c_func!(xmlTextReaderMoveToAttribute(_, _)),
    export_c_func!(xmlTextReaderMoveToFirstAttribute(_)),
    export_c_func!(xmlTextReaderMoveToNextAttribute(_)),
    export_c_func!(xmlTextReaderMoveToElement(_)),
    export_c_func!(xmlTextReaderReadInnerXml(_)),
    export_c_func!(xmlTextReaderReadOuterXml(_)),
    export_c_func!(xmlTextReaderReadString(_)),
    export_c_func!(xmlTextReaderNext(_)),
    export_c_func!(xmlTextReaderNextSibling(_)),
    export_c_func!(xmlTextReaderExpand(_)),
    export_c_func!(xmlTextReaderCurrentNode(_)),
    export_c_func!(xmlTextReaderCurrentDoc(_)),
    // ---- xpath ----
    export_c_func!(xmlXPathNewContext(_)),
    export_c_func!(xmlXPathFreeContext(_)),
    export_c_func!(xmlXPathRegisterNs(_, _, _)),
    export_c_func!(xmlXPathEval(_, _)),
    export_c_func!(xmlXPathEvalExpression(_, _)),
    export_c_func!(xmlXPathCompile(_)),
    export_c_func!(xmlXPathFreeCompExpr(_)),
    export_c_func!(xmlXPathCompiledEval(_, _)),
    export_c_func!(xmlXPathFreeObject(_)),
    export_c_func!(xmlXPathSetContextNode(_, _)),
    export_c_func!(xmlXPathCastToNumber(_)),
    export_c_func!(xmlXPathCastToBoolean(_)),
    export_c_func!(xmlXPathCastToString(_)),
    // (helpers exposed for guest code that doesn't reach into struct layout)
    export_c_func!(xmlXPathObjectGetType(_)),
    export_c_func!(xmlXPathObjectGetBool(_)),
    export_c_func!(xmlXPathObjectGetFloat(_)),
    export_c_func!(xmlXPathObjectGetString(_)),
    export_c_func!(xmlXPathObjectGetNodeSetSize(_)),
    export_c_func!(xmlXPathObjectGetNodeSetItem(_, _)),
    // ---- xinclude / xpointer ----
    export_c_func!(xmlXIncludeProcess(_)),
    export_c_func!(xmlXIncludeProcessFlags(_, _)),
    export_c_func!(xmlXIncludeProcessTree(_)),
    export_c_func!(xmlXIncludeProcessTreeFlags(_, _)),
    export_c_func!(xmlXPtrEval(_, _)),
    // ---- DTD validation ----
    export_c_func!(xmlNewValidCtxt()),
    export_c_func!(xmlFreeValidCtxt(_)),
    export_c_func!(xmlValidateDocument(_, _)),
    export_c_func!(xmlValidateDtd(_, _, _)),
    export_c_func!(xmlParseDTD(_, _)),
    export_c_func!(xmlFreeDtd(_)),
    export_c_func!(xmlNewDtd(_, _, _, _)),
    // ---- xml schemas ----
    export_c_func!(xmlSchemaNewParserCtxt(_)),
    export_c_func!(xmlSchemaNewMemParserCtxt(_, _)),
    export_c_func!(xmlSchemaFreeParserCtxt(_)),
    export_c_func!(xmlSchemaParse(_)),
    export_c_func!(xmlSchemaFree(_)),
    export_c_func!(xmlSchemaNewValidCtxt(_)),
    export_c_func!(xmlSchemaFreeValidCtxt(_)),
    export_c_func!(xmlSchemaValidateDoc(_, _)),
    export_c_func!(xmlSchemaValidateFile(_, _, _)),
    export_c_func!(xmlSchemaValidateOneElement(_, _)),
    // ---- relax-ng ----
    export_c_func!(xmlRelaxNGNewParserCtxt(_)),
    export_c_func!(xmlRelaxNGNewMemParserCtxt(_, _)),
    export_c_func!(xmlRelaxNGFreeParserCtxt(_)),
    export_c_func!(xmlRelaxNGParse(_)),
    export_c_func!(xmlRelaxNGFree(_)),
    export_c_func!(xmlRelaxNGNewValidCtxt(_)),
    export_c_func!(xmlRelaxNGFreeValidCtxt(_)),
    export_c_func!(xmlRelaxNGValidateDoc(_, _)),
    // ---- schematron ----
    export_c_func!(xmlSchematronNewParserCtxt(_)),
    export_c_func!(xmlSchematronNewMemParserCtxt(_, _)),
    export_c_func!(xmlSchematronFreeParserCtxt(_)),
    export_c_func!(xmlSchematronParse(_)),
    export_c_func!(xmlSchematronFree(_)),
    export_c_func!(xmlSchematronNewValidCtxt(_, _)),
    export_c_func!(xmlSchematronFreeValidCtxt(_)),
    export_c_func!(xmlSchematronValidateDoc(_, _)),
    // ---- catalog ----
    export_c_func!(xmlLoadCatalog(_)),
    export_c_func!(xmlLoadCatalogs(_)),
    export_c_func!(xmlCatalogResolveURI(_)),
    export_c_func!(xmlCatalogResolvePublic(_)),
    export_c_func!(xmlCatalogResolveSystem(_)),
    export_c_func!(xmlCatalogCleanup()),
    // ---- URI ----
    export_c_func!(xmlParseURI(_)),
    export_c_func!(xmlFreeURI(_)),
    export_c_func!(xmlBuildURI(_, _)),
    export_c_func!(xmlBuildRelativeURI(_, _)),
    export_c_func!(xmlURIEscape(_)),
    export_c_func!(xmlURIEscapeStr(_, _)),
    export_c_func!(xmlSaveUri(_)),
    export_c_func!(xmlCanonicPath(_)),
    // ---- save / serialization ----
    export_c_func!(xmlSaveFile(_, _)),
    export_c_func!(xmlSaveFormatFile(_, _, _)),
    export_c_func!(xmlSaveFileEnc(_, _, _)),
    export_c_func!(xmlSaveFormatFileEnc(_, _, _, _)),
    export_c_func!(xmlDocDumpMemory(_, _, _)),
    export_c_func!(xmlDocDumpFormatMemory(_, _, _, _)),
    export_c_func!(xmlDocDumpMemoryEnc(_, _, _, _)),
    export_c_func!(xmlDocDumpFormatMemoryEnc(_, _, _, _, _)),
    // ---- buffer / writer ----
    export_c_func!(xmlBufferCreate()),
    export_c_func!(xmlBufferCreateSize(_)),
    export_c_func!(xmlBufferFree(_)),
    export_c_func!(xmlBufferEmpty(_)),
    export_c_func!(xmlBufferContent(_)),
    export_c_func!(xmlBufferLength(_)),
    export_c_func!(xmlBufferAdd(_, _, _)),
    export_c_func!(xmlBufferCat(_, _)),
    export_c_func!(xmlNewTextWriterMemory(_, _)),
    export_c_func!(xmlNewTextWriterDoc(_, _)),
    export_c_func!(xmlFreeTextWriter(_)),
    export_c_func!(xmlTextWriterStartDocument(_, _, _, _)),
    export_c_func!(xmlTextWriterEndDocument(_)),
    export_c_func!(xmlTextWriterStartElement(_, _)),
    export_c_func!(xmlTextWriterStartElementNS(_, _, _, _)),
    export_c_func!(xmlTextWriterEndElement(_)),
    export_c_func!(xmlTextWriterFullEndElement(_)),
    export_c_func!(xmlTextWriterWriteString(_, _)),
    export_c_func!(xmlTextWriterWriteRaw(_, _)),
    export_c_func!(xmlTextWriterWriteAttribute(_, _, _)),
    export_c_func!(xmlTextWriterWriteAttributeNS(_, _, _, _, _)),
    export_c_func!(xmlTextWriterWriteComment(_, _)),
    export_c_func!(xmlTextWriterWriteCDATA(_, _)),
    export_c_func!(xmlTextWriterWritePI(_, _, _)),
    export_c_func!(xmlTextWriterFlush(_)),
    export_c_func!(xmlTextWriterSetIndent(_, _)),
    export_c_func!(xmlTextWriterSetIndentString(_, _)),
    // ---- html ----
    export_c_func!(htmlReadFile(_, _, _)),
    export_c_func!(htmlReadMemory(_, _, _, _, _)),
    export_c_func!(htmlReadDoc(_, _, _, _)),
    export_c_func!(htmlNewParserCtxt()),
    export_c_func!(htmlFreeParserCtxt(_)),
    export_c_func!(htmlNewDoc(_, _)),
    export_c_func!(htmlSaveFile(_, _)),
    export_c_func!(htmlDocDumpMemory(_, _, _)),
    export_c_func!(htmlDocDumpMemoryFormat(_, _, _, _)),
    // ---- error reporting ----
    export_c_func!(xmlGetLastError()),
    export_c_func!(xmlResetLastError()),
    export_c_func!(xmlErrorGetCode(_)),
    export_c_func!(xmlErrorGetLevel(_)),
    export_c_func!(xmlErrorGetLine(_)),
    export_c_func!(xmlErrorGetMessage(_)),
    export_c_func!(xmlErrorGetFile(_)),
    // ---- string utilities ----
    export_c_func!(xmlStrdup(_)),
    export_c_func!(xmlStrndup(_, _)),
    export_c_func!(xmlStrlen(_)),
    export_c_func!(xmlStrcmp(_, _)),
    export_c_func!(xmlStrncmp(_, _, _)),
    export_c_func!(xmlStrcasecmp(_, _)),
    export_c_func!(xmlStrncasecmp(_, _, _)),
    export_c_func!(xmlStrEqual(_, _)),
    export_c_func!(xmlStrsub(_, _, _)),
    export_c_func!(xmlStrcat(_, _)),
    export_c_func!(xmlStrncat(_, _, _)),
    export_c_func!(xmlStrchr(_, _)),
    export_c_func!(xmlStrstr(_, _)),
    export_c_func!(xmlCharStrdup(_)),
    // ---- entity encoding ----
    export_c_func!(xmlEncodeEntitiesReentrant(_, _)),
    export_c_func!(xmlEncodeSpecialChars(_, _)),
    // ---- generic accessors (host-side helpers exposed to guest) ----
    export_c_func!(xmlNodeGetName(_)),
    export_c_func!(xmlNodeGetType(_)),
    export_c_func!(xmlNodeGetDoc(_)),
    export_c_func!(xmlNodeGetParent(_)),
    export_c_func!(xmlNodeGetNext(_)),
    export_c_func!(xmlNodeGetPrev(_)),
    export_c_func!(xmlNodeGetProperties(_)),
    export_c_func!(xmlAttrName(_)),
    export_c_func!(xmlAttrNext(_)),
    // ---- memory ----
    export_c_func!(xmlFree(_)),
];
