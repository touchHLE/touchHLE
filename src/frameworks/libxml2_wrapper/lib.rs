/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Raw FFI bindings to the vendored libxml2 plus a tiny C shim
//! ([`shim.c`](./shim.c)) that exposes safe struct-field accessors.
//!
//! This crate is intentionally a thin pass-through: the higher-level
//! host-shim for the iPhone OS apps lives in `src/frameworks/libxml2.rs`
//! in the main crate.  Keeping the cmake build and the bindings in their
//! own crate matches the pattern used by `touchHLE_dynarmic_wrapper` and
//! `touchHLE_openal_soft_wrapper`.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use std::os::raw::{c_char, c_double, c_int, c_uchar, c_uint, c_ulong, c_void};

// ============================================================================
// Opaque types — everything libxml2 hands back is treated as an opaque pointer
// inside Rust.  Real struct field accesses go through the `hxml_*` shim.
// ============================================================================

pub type xmlChar = c_uchar;

#[repr(C)]
pub struct xmlDoc {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlNode {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlAttr {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlNs {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlDtd {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlEntity {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlParserCtxt {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlValidCtxt {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlSchema {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlSchemaParserCtxt {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlSchemaValidCtxt {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlRelaxNG {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlRelaxNGParserCtxt {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlRelaxNGValidCtxt {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlSchematron {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlSchematronParserCtxt {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlSchematronValidCtxt {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlXPathContext {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlXPathCompExpr {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlXPathObject {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlNodeSet {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlTextReader {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlTextWriter {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlSaveCtxt {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlBuffer {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlParserInputBuffer {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlOutputBuffer {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlURI {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlCatalog {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlError {
    _private: [u8; 0],
}
#[repr(C)]
pub struct xmlHashTable {
    _private: [u8; 0],
}
#[repr(C)]
pub struct htmlParserCtxt {
    _private: [u8; 0],
}

// ============================================================================
// libxml2 enums we care about
// ============================================================================

/// `xmlElementType` — see `include/libxml/tree.h`
pub mod element_type {
    pub const XML_ELEMENT_NODE: i32 = 1;
    pub const XML_ATTRIBUTE_NODE: i32 = 2;
    pub const XML_TEXT_NODE: i32 = 3;
    pub const XML_CDATA_SECTION_NODE: i32 = 4;
    pub const XML_ENTITY_REF_NODE: i32 = 5;
    pub const XML_ENTITY_NODE: i32 = 6;
    pub const XML_PI_NODE: i32 = 7;
    pub const XML_COMMENT_NODE: i32 = 8;
    pub const XML_DOCUMENT_NODE: i32 = 9;
    pub const XML_DOCUMENT_TYPE_NODE: i32 = 10;
    pub const XML_DOCUMENT_FRAG_NODE: i32 = 11;
    pub const XML_NOTATION_NODE: i32 = 12;
    pub const XML_HTML_DOCUMENT_NODE: i32 = 13;
    pub const XML_DTD_NODE: i32 = 14;
    pub const XML_ELEMENT_DECL: i32 = 15;
    pub const XML_ATTRIBUTE_DECL: i32 = 16;
    pub const XML_ENTITY_DECL: i32 = 17;
    pub const XML_NAMESPACE_DECL: i32 = 18;
    pub const XML_XINCLUDE_START: i32 = 19;
    pub const XML_XINCLUDE_END: i32 = 20;
}

/// `xmlXPathObjectType`
pub mod xpath_object_type {
    pub const XPATH_UNDEFINED: i32 = 0;
    pub const XPATH_NODESET: i32 = 1;
    pub const XPATH_BOOLEAN: i32 = 2;
    pub const XPATH_NUMBER: i32 = 3;
    pub const XPATH_STRING: i32 = 4;
    pub const XPATH_POINT: i32 = 5;
    pub const XPATH_RANGE: i32 = 6;
    pub const XPATH_LOCATIONSET: i32 = 7;
    pub const XPATH_USERS: i32 = 8;
    pub const XPATH_XSLT_TREE: i32 = 9;
}

/// `xmlReaderTypes`
pub mod reader_type {
    pub const XML_READER_TYPE_NONE: i32 = 0;
    pub const XML_READER_TYPE_ELEMENT: i32 = 1;
    pub const XML_READER_TYPE_ATTRIBUTE: i32 = 2;
    pub const XML_READER_TYPE_TEXT: i32 = 3;
    pub const XML_READER_TYPE_CDATA: i32 = 4;
    pub const XML_READER_TYPE_ENTITY_REFERENCE: i32 = 5;
    pub const XML_READER_TYPE_ENTITY: i32 = 6;
    pub const XML_READER_TYPE_PROCESSING_INSTRUCTION: i32 = 7;
    pub const XML_READER_TYPE_COMMENT: i32 = 8;
    pub const XML_READER_TYPE_DOCUMENT: i32 = 9;
    pub const XML_READER_TYPE_DOCUMENT_TYPE: i32 = 10;
    pub const XML_READER_TYPE_DOCUMENT_FRAGMENT: i32 = 11;
    pub const XML_READER_TYPE_NOTATION: i32 = 12;
    pub const XML_READER_TYPE_WHITESPACE: i32 = 13;
    pub const XML_READER_TYPE_SIGNIFICANT_WHITESPACE: i32 = 14;
    pub const XML_READER_TYPE_END_ELEMENT: i32 = 15;
    pub const XML_READER_TYPE_END_ENTITY: i32 = 16;
    pub const XML_READER_TYPE_XML_DECLARATION: i32 = 17;
}

/// `xmlParserOption`
pub mod parser_option {
    pub const XML_PARSE_RECOVER: i32 = 1 << 0;
    pub const XML_PARSE_NOENT: i32 = 1 << 1;
    pub const XML_PARSE_DTDLOAD: i32 = 1 << 2;
    pub const XML_PARSE_DTDATTR: i32 = 1 << 3;
    pub const XML_PARSE_DTDVALID: i32 = 1 << 4;
    pub const XML_PARSE_NOERROR: i32 = 1 << 5;
    pub const XML_PARSE_NOWARNING: i32 = 1 << 6;
    pub const XML_PARSE_PEDANTIC: i32 = 1 << 7;
    pub const XML_PARSE_NOBLANKS: i32 = 1 << 8;
    pub const XML_PARSE_SAX1: i32 = 1 << 9;
    pub const XML_PARSE_XINCLUDE: i32 = 1 << 10;
    pub const XML_PARSE_NONET: i32 = 1 << 11;
    pub const XML_PARSE_NODICT: i32 = 1 << 12;
    pub const XML_PARSE_NSCLEAN: i32 = 1 << 13;
    pub const XML_PARSE_NOCDATA: i32 = 1 << 14;
    pub const XML_PARSE_NOXINCNODE: i32 = 1 << 15;
    pub const XML_PARSE_COMPACT: i32 = 1 << 16;
    pub const XML_PARSE_OLD10: i32 = 1 << 17;
    pub const XML_PARSE_NOBASEFIX: i32 = 1 << 18;
    pub const XML_PARSE_HUGE: i32 = 1 << 19;
    pub const XML_PARSE_OLDSAX: i32 = 1 << 20;
    pub const XML_PARSE_IGNORE_ENC: i32 = 1 << 21;
    pub const XML_PARSE_BIG_LINES: i32 = 1 << 22;
}

/// `htmlParserOption` (subset)
pub mod html_parser_option {
    pub const HTML_PARSE_RECOVER: i32 = 1 << 0;
    pub const HTML_PARSE_NODEFDTD: i32 = 1 << 2;
    pub const HTML_PARSE_NOERROR: i32 = 1 << 5;
    pub const HTML_PARSE_NOWARNING: i32 = 1 << 6;
    pub const HTML_PARSE_PEDANTIC: i32 = 1 << 7;
    pub const HTML_PARSE_NOBLANKS: i32 = 1 << 8;
    pub const HTML_PARSE_NONET: i32 = 1 << 11;
    pub const HTML_PARSE_NOIMPLIED: i32 = 1 << 13;
    pub const HTML_PARSE_COMPACT: i32 = 1 << 16;
    pub const HTML_PARSE_IGNORE_ENC: i32 = 1 << 21;
}

// ============================================================================
// extern "C" — libxml2 + shim functions
// ============================================================================

extern "C" {
    // ------------------------ One-time setup ----------------------------
    pub fn hxml_init();
    pub fn hxml_cleanup();
    pub fn hxml_install_quiet_error_handlers();
    pub fn xmlInitParser();
    pub fn xmlCleanupParser();
    pub fn xmlKeepBlanksDefault(val: c_int) -> c_int;
    pub fn xmlCheckVersion(version: c_int);

    // ------------------------ Memory ------------------------------------
    pub fn xmlFree(ptr: *mut c_void);
    pub fn hxml_free(ptr: *mut c_void);
    pub fn xmlMalloc(size: usize) -> *mut c_void;
    pub fn xmlMemoryDump() -> c_int;
    /// `xmlInitMemory(void)` — initialises libxml2's allocator override
    /// table. Deprecated upstream (the runtime now initialises lazily),
    /// but apps built against older libxml2 still call it.
    pub fn xmlInitMemory() -> c_int;
    /// `xmlCleanupMemory(void)` — releases any module-private state held
    /// by the libxml2 allocator. See `libxml/xmlmemory.h`.
    pub fn xmlCleanupMemory();

    // ------------------------ xmlChar string utilities ------------------
    pub fn xmlStrdup(s: *const xmlChar) -> *mut xmlChar;
    pub fn xmlStrndup(s: *const xmlChar, len: c_int) -> *mut xmlChar;
    pub fn xmlStrlen(s: *const xmlChar) -> c_int;
    pub fn xmlStrcmp(a: *const xmlChar, b: *const xmlChar) -> c_int;
    pub fn xmlStrncmp(a: *const xmlChar, b: *const xmlChar, n: c_int) -> c_int;
    pub fn xmlStrcasecmp(a: *const xmlChar, b: *const xmlChar) -> c_int;
    pub fn xmlStrncasecmp(a: *const xmlChar, b: *const xmlChar, n: c_int) -> c_int;
    pub fn xmlStrEqual(a: *const xmlChar, b: *const xmlChar) -> c_int;
    pub fn xmlStrcat(a: *mut xmlChar, b: *const xmlChar) -> *mut xmlChar;
    pub fn xmlStrncat(a: *mut xmlChar, b: *const xmlChar, n: c_int) -> *mut xmlChar;
    pub fn xmlStrsub(s: *const xmlChar, start: c_int, len: c_int) -> *mut xmlChar;
    pub fn xmlStrchr(s: *const xmlChar, ch: xmlChar) -> *const xmlChar;
    pub fn xmlStrstr(s: *const xmlChar, needle: *const xmlChar) -> *const xmlChar;
    pub fn xmlCharStrdup(s: *const c_char) -> *mut xmlChar;

    // ------------------------ Parser ------------------------------------
    pub fn xmlNewParserCtxt() -> *mut xmlParserCtxt;
    pub fn xmlFreeParserCtxt(ctxt: *mut xmlParserCtxt);
    pub fn xmlClearParserCtxt(ctxt: *mut xmlParserCtxt);
    pub fn xmlCtxtReset(ctxt: *mut xmlParserCtxt);
    pub fn xmlCtxtReadFile(
        ctxt: *mut xmlParserCtxt,
        filename: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlDoc;
    pub fn xmlCtxtReadMemory(
        ctxt: *mut xmlParserCtxt,
        buf: *const c_char,
        size: c_int,
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlDoc;
    pub fn xmlCtxtReadDoc(
        ctxt: *mut xmlParserCtxt,
        cur: *const xmlChar,
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlDoc;
    pub fn xmlReadFile(
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlDoc;
    pub fn xmlReadMemory(
        buf: *const c_char,
        size: c_int,
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlDoc;
    pub fn xmlReadDoc(
        cur: *const xmlChar,
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlDoc;
    pub fn xmlParseFile(filename: *const c_char) -> *mut xmlDoc;
    pub fn xmlParseMemory(buf: *const c_char, size: c_int) -> *mut xmlDoc;
    pub fn xmlParseDoc(cur: *const xmlChar) -> *mut xmlDoc;
    pub fn xmlCtxtGetLastError(ctxt: *mut c_void) -> *mut xmlError;
    pub fn xmlCtxtResetLastError(ctxt: *mut c_void);

    // ------------------------ Push parser -------------------------------
    pub fn xmlCreatePushParserCtxt(
        sax: *mut c_void,
        user_data: *mut c_void,
        chunk: *const c_char,
        size: c_int,
        filename: *const c_char,
    ) -> *mut xmlParserCtxt;
    pub fn xmlParseChunk(
        ctxt: *mut xmlParserCtxt,
        chunk: *const c_char,
        size: c_int,
        terminate: c_int,
    ) -> c_int;

    // ------------------------ SAX parser -------------------------------
    pub fn xmlSAXUserParseMemory(
        sax: *mut c_void,
        user_data: *mut c_void,
        buffer: *const c_char,
        size: c_int,
    ) -> c_int;

    // ------------------------ Tree --------------------------------------
    pub fn xmlNewDoc(version: *const xmlChar) -> *mut xmlDoc;
    pub fn xmlFreeDoc(doc: *mut xmlDoc);
    pub fn xmlDocGetRootElement(doc: *mut xmlDoc) -> *mut xmlNode;
    pub fn xmlDocSetRootElement(doc: *mut xmlDoc, root: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlNewNode(ns: *mut xmlNs, name: *const xmlChar) -> *mut xmlNode;
    pub fn xmlNewDocNode(
        doc: *mut xmlDoc,
        ns: *mut xmlNs,
        name: *const xmlChar,
        content: *const xmlChar,
    ) -> *mut xmlNode;
    pub fn xmlNewText(content: *const xmlChar) -> *mut xmlNode;
    pub fn xmlNewDocText(doc: *mut xmlDoc, content: *const xmlChar) -> *mut xmlNode;
    pub fn xmlNewComment(content: *const xmlChar) -> *mut xmlNode;
    pub fn xmlNewCDataBlock(doc: *mut xmlDoc, content: *const xmlChar, len: c_int) -> *mut xmlNode;
    pub fn xmlNewChild(
        parent: *mut xmlNode,
        ns: *mut xmlNs,
        name: *const xmlChar,
        content: *const xmlChar,
    ) -> *mut xmlNode;
    pub fn xmlNewTextChild(
        parent: *mut xmlNode,
        ns: *mut xmlNs,
        name: *const xmlChar,
        content: *const xmlChar,
    ) -> *mut xmlNode;
    pub fn xmlAddChild(parent: *mut xmlNode, cur: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlAddPrevSibling(cur: *mut xmlNode, elem: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlAddNextSibling(cur: *mut xmlNode, elem: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlAddSibling(cur: *mut xmlNode, elem: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlUnlinkNode(cur: *mut xmlNode);
    pub fn xmlFreeNode(cur: *mut xmlNode);
    pub fn xmlFreeNodeList(cur: *mut xmlNode);
    pub fn xmlCopyNode(node: *mut xmlNode, recursive: c_int) -> *mut xmlNode;
    pub fn xmlDocCopyNode(
        node: *mut xmlNode,
        doc: *mut xmlDoc,
        recursive: c_int,
    ) -> *mut xmlNode;
    pub fn xmlReplaceNode(old: *mut xmlNode, cur: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlNodeSetName(cur: *mut xmlNode, name: *const xmlChar);
    pub fn xmlNodeSetContent(cur: *mut xmlNode, content: *const xmlChar);
    pub fn xmlNodeSetContentLen(cur: *mut xmlNode, content: *const xmlChar, len: c_int);
    pub fn xmlNodeAddContent(cur: *mut xmlNode, content: *const xmlChar);
    pub fn xmlNodeAddContentLen(cur: *mut xmlNode, content: *const xmlChar, len: c_int);
    pub fn xmlNodeGetContent(cur: *mut xmlNode) -> *mut xmlChar;
    pub fn xmlNodeDump(
        buf: *mut xmlBuffer,
        doc: *mut xmlDoc,
        cur: *mut xmlNode,
        level: c_int,
        format: c_int,
    ) -> c_int;
    pub fn xmlNodeListGetString(
        doc: *mut xmlDoc,
        list: *mut xmlNode,
        inLine: c_int,
    ) -> *mut xmlChar;
    pub fn xmlGetLastChild(node: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlChildElementCount(parent: *mut xmlNode) -> c_ulong;
    pub fn xmlFirstElementChild(parent: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlLastElementChild(parent: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlNextElementSibling(node: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlPreviousElementSibling(node: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlGetNodePath(node: *mut xmlNode) -> *mut xmlChar;
    pub fn xmlGetLineNo(node: *mut xmlNode) -> c_int;

    // ------------------------ Attributes / Namespaces -------------------
    pub fn xmlNewProp(
        node: *mut xmlNode,
        name: *const xmlChar,
        value: *const xmlChar,
    ) -> *mut xmlAttr;
    pub fn xmlNewNsProp(
        node: *mut xmlNode,
        ns: *mut xmlNs,
        name: *const xmlChar,
        value: *const xmlChar,
    ) -> *mut xmlAttr;
    pub fn xmlSetProp(
        node: *mut xmlNode,
        name: *const xmlChar,
        value: *const xmlChar,
    ) -> *mut xmlAttr;
    pub fn xmlSetNsProp(
        node: *mut xmlNode,
        ns: *mut xmlNs,
        name: *const xmlChar,
        value: *const xmlChar,
    ) -> *mut xmlAttr;
    pub fn xmlUnsetProp(node: *mut xmlNode, name: *const xmlChar) -> c_int;
    pub fn xmlUnsetNsProp(node: *mut xmlNode, ns: *mut xmlNs, name: *const xmlChar) -> c_int;
    pub fn xmlGetProp(node: *mut xmlNode, name: *const xmlChar) -> *mut xmlChar;
    pub fn xmlGetNoNsProp(node: *mut xmlNode, name: *const xmlChar) -> *mut xmlChar;
    pub fn xmlGetNsProp(
        node: *mut xmlNode,
        name: *const xmlChar,
        ns: *const xmlChar,
    ) -> *mut xmlChar;
    pub fn xmlHasProp(node: *mut xmlNode, name: *const xmlChar) -> *mut xmlAttr;
    pub fn xmlHasNsProp(
        node: *mut xmlNode,
        name: *const xmlChar,
        ns: *const xmlChar,
    ) -> *mut xmlAttr;
    pub fn xmlNewNs(node: *mut xmlNode, href: *const xmlChar, prefix: *const xmlChar) -> *mut xmlNs;
    pub fn xmlSearchNs(
        doc: *mut xmlDoc,
        node: *mut xmlNode,
        prefix: *const xmlChar,
    ) -> *mut xmlNs;
    pub fn xmlSearchNsByHref(
        doc: *mut xmlDoc,
        node: *mut xmlNode,
        href: *const xmlChar,
    ) -> *mut xmlNs;
    pub fn xmlSetNs(node: *mut xmlNode, ns: *mut xmlNs);
    pub fn xmlFreeNs(ns: *mut xmlNs);

    // ------------------------ TextReader (xmlreader.h) ------------------
    pub fn xmlReaderForFile(
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlTextReader;
    pub fn xmlReaderForMemory(
        buffer: *const c_char,
        size: c_int,
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlTextReader;
    pub fn xmlReaderForDoc(
        cur: *const xmlChar,
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlTextReader;
    pub fn xmlFreeTextReader(reader: *mut xmlTextReader);
    pub fn xmlTextReaderRead(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderClose(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderNodeType(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderDepth(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderAttributeCount(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderHasAttributes(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderHasValue(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderIsEmptyElement(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderName(reader: *mut xmlTextReader) -> *mut xmlChar;
    pub fn xmlTextReaderLocalName(reader: *mut xmlTextReader) -> *mut xmlChar;
    pub fn xmlTextReaderPrefix(reader: *mut xmlTextReader) -> *mut xmlChar;
    pub fn xmlTextReaderNamespaceUri(reader: *mut xmlTextReader) -> *mut xmlChar;
    pub fn xmlTextReaderValue(reader: *mut xmlTextReader) -> *mut xmlChar;
    pub fn xmlTextReaderConstName(reader: *mut xmlTextReader) -> *const xmlChar;
    pub fn xmlTextReaderConstLocalName(reader: *mut xmlTextReader) -> *const xmlChar;
    pub fn xmlTextReaderConstPrefix(reader: *mut xmlTextReader) -> *const xmlChar;
    pub fn xmlTextReaderConstNamespaceUri(reader: *mut xmlTextReader) -> *const xmlChar;
    pub fn xmlTextReaderConstValue(reader: *mut xmlTextReader) -> *const xmlChar;
    pub fn xmlTextReaderConstString(
        reader: *mut xmlTextReader,
        str: *const xmlChar,
    ) -> *const xmlChar;
    pub fn xmlTextReaderGetAttribute(
        reader: *mut xmlTextReader,
        name: *const xmlChar,
    ) -> *mut xmlChar;
    pub fn xmlTextReaderGetAttributeNo(reader: *mut xmlTextReader, no: c_int) -> *mut xmlChar;
    pub fn xmlTextReaderGetAttributeNs(
        reader: *mut xmlTextReader,
        local_name: *const xmlChar,
        namespace_uri: *const xmlChar,
    ) -> *mut xmlChar;
    pub fn xmlTextReaderMoveToAttribute(
        reader: *mut xmlTextReader,
        name: *const xmlChar,
    ) -> c_int;
    pub fn xmlTextReaderMoveToFirstAttribute(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderMoveToNextAttribute(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderMoveToElement(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderReadInnerXml(reader: *mut xmlTextReader) -> *mut xmlChar;
    pub fn xmlTextReaderReadOuterXml(reader: *mut xmlTextReader) -> *mut xmlChar;
    pub fn xmlTextReaderReadString(reader: *mut xmlTextReader) -> *mut xmlChar;
    pub fn xmlTextReaderNext(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderNextSibling(reader: *mut xmlTextReader) -> c_int;
    pub fn xmlTextReaderExpand(reader: *mut xmlTextReader) -> *mut xmlNode;
    pub fn xmlTextReaderCurrentNode(reader: *mut xmlTextReader) -> *mut xmlNode;
    pub fn xmlTextReaderCurrentDoc(reader: *mut xmlTextReader) -> *mut xmlDoc;

    // ------------------------ XPath (xpath.h) ---------------------------
    pub fn xmlXPathNewContext(doc: *mut xmlDoc) -> *mut xmlXPathContext;
    pub fn xmlXPathFreeContext(ctx: *mut xmlXPathContext);
    pub fn xmlXPathRegisterNs(
        ctx: *mut xmlXPathContext,
        prefix: *const xmlChar,
        ns_uri: *const xmlChar,
    ) -> c_int;
    pub fn xmlXPathRegisteredNsCleanup(ctx: *mut xmlXPathContext);
    pub fn xmlXPathEval(
        expr: *const xmlChar,
        ctx: *mut xmlXPathContext,
    ) -> *mut xmlXPathObject;
    pub fn xmlXPathEvalExpression(
        expr: *const xmlChar,
        ctx: *mut xmlXPathContext,
    ) -> *mut xmlXPathObject;
    pub fn xmlXPathCompile(expr: *const xmlChar) -> *mut xmlXPathCompExpr;
    pub fn xmlXPathFreeCompExpr(expr: *mut xmlXPathCompExpr);
    pub fn xmlXPathCompiledEval(
        expr: *mut xmlXPathCompExpr,
        ctx: *mut xmlXPathContext,
    ) -> *mut xmlXPathObject;
    pub fn xmlXPathFreeObject(obj: *mut xmlXPathObject);
    pub fn xmlXPathNodeSetCreate(node: *mut xmlNode) -> *mut xmlNodeSet;
    pub fn xmlXPathFreeNodeSet(set: *mut xmlNodeSet);
    pub fn xmlXPathSetContextNode(node: *mut xmlNode, ctx: *mut xmlXPathContext) -> c_int;
    /// `xmlXPathCastToNumber(const xmlXPathObjectPtr val) -> double` —
    /// XPath 1.0 §4.4 number conversion: numbers pass through unchanged,
    /// booleans map to 1/0, node-sets to the number value of the first
    /// node in document order, strings via `xmlXPathCastStringToNumber`.
    pub fn xmlXPathCastToNumber(val: *mut xmlXPathObject) -> c_double;
    pub fn xmlXPathCastToString(val: *mut xmlXPathObject) -> *mut xmlChar;
    pub fn xmlXPathCastToBoolean(val: *mut xmlXPathObject) -> c_int;

    // ------------------------ XInclude / XPointer -----------------------
    pub fn xmlXIncludeProcess(doc: *mut xmlDoc) -> c_int;
    pub fn xmlXIncludeProcessFlags(doc: *mut xmlDoc, flags: c_int) -> c_int;
    pub fn xmlXIncludeProcessTree(node: *mut xmlNode) -> c_int;
    pub fn xmlXIncludeProcessTreeFlags(node: *mut xmlNode, flags: c_int) -> c_int;
    pub fn xmlXPtrEval(expr: *const xmlChar, ctx: *mut xmlXPathContext) -> *mut xmlXPathObject;

    // ------------------------ DTD validation (valid.h) ------------------
    pub fn xmlNewValidCtxt() -> *mut xmlValidCtxt;
    pub fn xmlFreeValidCtxt(ctxt: *mut xmlValidCtxt);
    pub fn xmlValidateDocument(ctxt: *mut xmlValidCtxt, doc: *mut xmlDoc) -> c_int;
    pub fn xmlValidateDtd(
        ctxt: *mut xmlValidCtxt,
        doc: *mut xmlDoc,
        dtd: *mut xmlDtd,
    ) -> c_int;
    pub fn xmlParseDTD(externalID: *const xmlChar, systemID: *const xmlChar) -> *mut xmlDtd;
    pub fn xmlFreeDtd(cur: *mut xmlDtd);
    pub fn xmlNewDtd(
        doc: *mut xmlDoc,
        name: *const xmlChar,
        external_id: *const xmlChar,
        system_id: *const xmlChar,
    ) -> *mut xmlDtd;

    // ------------------------ XML Schemas (xmlschemas.h) ----------------
    pub fn xmlSchemaNewParserCtxt(url: *const c_char) -> *mut xmlSchemaParserCtxt;
    pub fn xmlSchemaNewMemParserCtxt(buf: *const c_char, size: c_int) -> *mut xmlSchemaParserCtxt;
    pub fn xmlSchemaFreeParserCtxt(ctxt: *mut xmlSchemaParserCtxt);
    pub fn xmlSchemaParse(ctxt: *mut xmlSchemaParserCtxt) -> *mut xmlSchema;
    pub fn xmlSchemaFree(schema: *mut xmlSchema);
    pub fn xmlSchemaNewValidCtxt(schema: *mut xmlSchema) -> *mut xmlSchemaValidCtxt;
    pub fn xmlSchemaFreeValidCtxt(ctxt: *mut xmlSchemaValidCtxt);
    pub fn xmlSchemaValidateDoc(ctxt: *mut xmlSchemaValidCtxt, doc: *mut xmlDoc) -> c_int;
    pub fn xmlSchemaValidateFile(
        ctxt: *mut xmlSchemaValidCtxt,
        filename: *const c_char,
        options: c_int,
    ) -> c_int;
    pub fn xmlSchemaValidateOneElement(
        ctxt: *mut xmlSchemaValidCtxt,
        elem: *mut xmlNode,
    ) -> c_int;

    // ------------------------ Relax-NG (relaxng.h) ----------------------
    pub fn xmlRelaxNGNewParserCtxt(url: *const c_char) -> *mut xmlRelaxNGParserCtxt;
    pub fn xmlRelaxNGNewMemParserCtxt(
        buf: *const c_char,
        size: c_int,
    ) -> *mut xmlRelaxNGParserCtxt;
    pub fn xmlRelaxNGFreeParserCtxt(ctxt: *mut xmlRelaxNGParserCtxt);
    pub fn xmlRelaxNGParse(ctxt: *mut xmlRelaxNGParserCtxt) -> *mut xmlRelaxNG;
    pub fn xmlRelaxNGFree(schema: *mut xmlRelaxNG);
    pub fn xmlRelaxNGNewValidCtxt(schema: *mut xmlRelaxNG) -> *mut xmlRelaxNGValidCtxt;
    pub fn xmlRelaxNGFreeValidCtxt(ctxt: *mut xmlRelaxNGValidCtxt);
    pub fn xmlRelaxNGValidateDoc(ctxt: *mut xmlRelaxNGValidCtxt, doc: *mut xmlDoc) -> c_int;

    // ------------------------ Schematron --------------------------------
    pub fn xmlSchematronNewParserCtxt(url: *const c_char) -> *mut xmlSchematronParserCtxt;
    pub fn xmlSchematronNewMemParserCtxt(
        buf: *const c_char,
        size: c_int,
    ) -> *mut xmlSchematronParserCtxt;
    pub fn xmlSchematronFreeParserCtxt(ctxt: *mut xmlSchematronParserCtxt);
    pub fn xmlSchematronParse(ctxt: *mut xmlSchematronParserCtxt) -> *mut xmlSchematron;
    pub fn xmlSchematronFree(schema: *mut xmlSchematron);
    pub fn xmlSchematronNewValidCtxt(
        schema: *mut xmlSchematron,
        options: c_int,
    ) -> *mut xmlSchematronValidCtxt;
    pub fn xmlSchematronFreeValidCtxt(ctxt: *mut xmlSchematronValidCtxt);
    pub fn xmlSchematronValidateDoc(
        ctxt: *mut xmlSchematronValidCtxt,
        doc: *mut xmlDoc,
    ) -> c_int;

    // ------------------------ Catalogs (catalog.h) ----------------------
    pub fn xmlLoadCatalog(filename: *const c_char) -> c_int;
    pub fn xmlLoadCatalogs(paths: *const c_char);
    pub fn xmlCatalogResolveURI(uri: *const xmlChar) -> *mut xmlChar;
    pub fn xmlCatalogResolvePublic(pub_id: *const xmlChar) -> *mut xmlChar;
    pub fn xmlCatalogResolveSystem(sys_id: *const xmlChar) -> *mut xmlChar;
    pub fn xmlCatalogCleanup();
    pub fn xmlNewCatalog(sgml: c_int) -> *mut xmlCatalog;
    pub fn xmlFreeCatalog(catal: *mut xmlCatalog);

    // ------------------------ URI (uri.h) -------------------------------
    pub fn xmlParseURI(uri: *const c_char) -> *mut xmlURI;
    pub fn xmlFreeURI(uri: *mut xmlURI);
    pub fn xmlBuildURI(uri: *const xmlChar, base: *const xmlChar) -> *mut xmlChar;
    pub fn xmlBuildRelativeURI(uri: *const xmlChar, base: *const xmlChar) -> *mut xmlChar;
    pub fn xmlURIEscape(s: *const xmlChar) -> *mut xmlChar;
    pub fn xmlURIEscapeStr(s: *const xmlChar, list: *const xmlChar) -> *mut xmlChar;
    pub fn xmlURIUnescapeString(s: *const c_char, len: c_int, target: *mut c_char) -> *mut c_char;
    pub fn xmlSaveUri(uri: *mut xmlURI) -> *mut xmlChar;
    pub fn xmlCanonicPath(path: *const xmlChar) -> *mut xmlChar;

    // ------------------------ Save / serialize (xmlsave.h, tree.h) -------
    pub fn xmlSaveFile(filename: *const c_char, cur: *mut xmlDoc) -> c_int;
    pub fn xmlSaveFormatFile(
        filename: *const c_char,
        cur: *mut xmlDoc,
        format: c_int,
    ) -> c_int;
    pub fn xmlSaveFileEnc(
        filename: *const c_char,
        cur: *mut xmlDoc,
        encoding: *const c_char,
    ) -> c_int;
    pub fn xmlSaveFormatFileEnc(
        filename: *const c_char,
        cur: *mut xmlDoc,
        encoding: *const c_char,
        format: c_int,
    ) -> c_int;
    pub fn xmlDocDumpMemory(doc: *mut xmlDoc, mem: *mut *mut xmlChar, size: *mut c_int);
    pub fn xmlDocDumpMemoryEnc(
        doc: *mut xmlDoc,
        mem: *mut *mut xmlChar,
        size: *mut c_int,
        encoding: *const c_char,
    );
    pub fn xmlDocDumpFormatMemory(
        doc: *mut xmlDoc,
        mem: *mut *mut xmlChar,
        size: *mut c_int,
        format: c_int,
    );
    pub fn xmlDocDumpFormatMemoryEnc(
        doc: *mut xmlDoc,
        mem: *mut *mut xmlChar,
        size: *mut c_int,
        encoding: *const c_char,
        format: c_int,
    );
    pub fn xmlSaveToFilename(
        filename: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlSaveCtxt;
    pub fn xmlSaveToBuffer(
        buffer: *mut xmlBuffer,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlSaveCtxt;
    pub fn xmlSaveDoc(ctx: *mut xmlSaveCtxt, doc: *mut xmlDoc) -> c_int;
    pub fn xmlSaveTree(ctx: *mut xmlSaveCtxt, node: *mut xmlNode) -> c_int;
    pub fn xmlSaveFlush(ctx: *mut xmlSaveCtxt) -> c_int;
    pub fn xmlSaveClose(ctx: *mut xmlSaveCtxt) -> c_int;

    // ------------------------ Writer (xmlwriter.h) -----------------------
    pub fn xmlNewTextWriterFilename(uri: *const c_char, compression: c_int) -> *mut xmlTextWriter;
    pub fn xmlNewTextWriterMemory(buf: *mut xmlBuffer, compression: c_int) -> *mut xmlTextWriter;
    pub fn xmlNewTextWriterDoc(doc: *mut *mut xmlDoc, compression: c_int) -> *mut xmlTextWriter;
    pub fn xmlFreeTextWriter(writer: *mut xmlTextWriter);
    pub fn xmlTextWriterStartDocument(
        writer: *mut xmlTextWriter,
        version: *const c_char,
        encoding: *const c_char,
        standalone: *const c_char,
    ) -> c_int;
    pub fn xmlTextWriterEndDocument(writer: *mut xmlTextWriter) -> c_int;
    pub fn xmlTextWriterStartElement(
        writer: *mut xmlTextWriter,
        name: *const xmlChar,
    ) -> c_int;
    pub fn xmlTextWriterStartElementNS(
        writer: *mut xmlTextWriter,
        prefix: *const xmlChar,
        name: *const xmlChar,
        ns_uri: *const xmlChar,
    ) -> c_int;
    pub fn xmlTextWriterEndElement(writer: *mut xmlTextWriter) -> c_int;
    pub fn xmlTextWriterFullEndElement(writer: *mut xmlTextWriter) -> c_int;
    pub fn xmlTextWriterWriteString(
        writer: *mut xmlTextWriter,
        content: *const xmlChar,
    ) -> c_int;
    pub fn xmlTextWriterWriteRaw(
        writer: *mut xmlTextWriter,
        content: *const xmlChar,
    ) -> c_int;
    pub fn xmlTextWriterWriteAttribute(
        writer: *mut xmlTextWriter,
        name: *const xmlChar,
        content: *const xmlChar,
    ) -> c_int;
    pub fn xmlTextWriterWriteAttributeNS(
        writer: *mut xmlTextWriter,
        prefix: *const xmlChar,
        name: *const xmlChar,
        ns_uri: *const xmlChar,
        content: *const xmlChar,
    ) -> c_int;
    pub fn xmlTextWriterWriteComment(
        writer: *mut xmlTextWriter,
        content: *const xmlChar,
    ) -> c_int;
    pub fn xmlTextWriterWriteCDATA(
        writer: *mut xmlTextWriter,
        content: *const xmlChar,
    ) -> c_int;
    pub fn xmlTextWriterWritePI(
        writer: *mut xmlTextWriter,
        target: *const xmlChar,
        content: *const xmlChar,
    ) -> c_int;
    pub fn xmlTextWriterFlush(writer: *mut xmlTextWriter) -> c_int;
    pub fn xmlTextWriterSetIndent(writer: *mut xmlTextWriter, indent: c_int) -> c_int;
    pub fn xmlTextWriterSetIndentString(
        writer: *mut xmlTextWriter,
        s: *const xmlChar,
    ) -> c_int;

    // ------------------------ Buffer ------------------------------------
    pub fn xmlBufferCreate() -> *mut xmlBuffer;
    pub fn xmlBufferCreateSize(size: usize) -> *mut xmlBuffer;
    pub fn xmlBufferFree(buf: *mut xmlBuffer);
    pub fn xmlBufferEmpty(buf: *mut xmlBuffer);
    pub fn xmlBufferContent(buf: *const xmlBuffer) -> *const xmlChar;
    pub fn xmlBufferLength(buf: *const xmlBuffer) -> c_int;
    pub fn xmlBufferAdd(buf: *mut xmlBuffer, s: *const xmlChar, len: c_int) -> c_int;
    pub fn xmlBufferCat(buf: *mut xmlBuffer, s: *const xmlChar) -> c_int;

    // ------------------------ HTML parser / tree -----------------------
    pub fn htmlReadFile(
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlDoc;
    pub fn htmlReadMemory(
        buf: *const c_char,
        size: c_int,
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlDoc;
    pub fn htmlReadDoc(
        cur: *const xmlChar,
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut xmlDoc;
    pub fn htmlNewParserCtxt() -> *mut htmlParserCtxt;
    pub fn htmlFreeParserCtxt(ctxt: *mut htmlParserCtxt);
    pub fn htmlNewDoc(uri: *const xmlChar, external_id: *const xmlChar) -> *mut xmlDoc;
    pub fn htmlSaveFile(filename: *const c_char, doc: *mut xmlDoc) -> c_int;
    pub fn htmlSaveFileEnc(
        filename: *const c_char,
        doc: *mut xmlDoc,
        encoding: *const c_char,
    ) -> c_int;
    pub fn htmlSaveFileFormat(
        filename: *const c_char,
        doc: *mut xmlDoc,
        encoding: *const c_char,
        format: c_int,
    ) -> c_int;
    pub fn htmlDocDumpMemory(doc: *mut xmlDoc, mem: *mut *mut xmlChar, size: *mut c_int);
    pub fn htmlDocDumpMemoryFormat(
        doc: *mut xmlDoc,
        mem: *mut *mut xmlChar,
        size: *mut c_int,
        format: c_int,
    );

    // ------------------------ Error reporting --------------------------
    pub fn xmlGetLastError() -> *mut xmlError;
    pub fn xmlResetLastError();
    pub fn xmlSetGenericErrorFunc(ctx: *mut c_void, handler: *mut c_void);
    pub fn xmlSetStructuredErrorFunc(ctx: *mut c_void, handler: *mut c_void);

    // ------------------------ Entities ---------------------------------
    pub fn xmlGetDocEntity(doc: *mut xmlDoc, name: *const xmlChar) -> *mut xmlEntity;
    pub fn xmlAddDocEntity(
        doc: *mut xmlDoc,
        name: *const xmlChar,
        kind: c_int,
        external_id: *const xmlChar,
        system_id: *const xmlChar,
        content: *const xmlChar,
    ) -> *mut xmlEntity;
    pub fn xmlEncodeEntitiesReentrant(doc: *mut xmlDoc, s: *const xmlChar) -> *mut xmlChar;
    pub fn xmlEncodeSpecialChars(doc: *mut xmlDoc, s: *const xmlChar) -> *mut xmlChar;

    // ------------------------ Threads ---------------------------------
    pub fn xmlInitThreads();
    pub fn xmlCleanupThreads();
    pub fn xmlLockLibrary();
    pub fn xmlUnlockLibrary();

    // ------------------------ C14N -----------------------------------
    pub fn xmlC14NDocSaveTo(
        doc: *mut xmlDoc,
        nodes: *mut xmlNodeSet,
        mode: c_int,
        inclusive_ns_prefixes: *mut *mut xmlChar,
        with_comments: c_int,
        buf: *mut xmlOutputBuffer,
    ) -> c_int;
    pub fn xmlC14NDocDumpMemory(
        doc: *mut xmlDoc,
        nodes: *mut xmlNodeSet,
        mode: c_int,
        inclusive_ns_prefixes: *mut *mut xmlChar,
        with_comments: c_int,
        doc_txt_ptr: *mut *mut xmlChar,
    ) -> c_int;

    // ------------------------ Version ---------------------------------
    pub static xmlParserVersion: *const c_char;
}

// ----- Shim accessors (defined in shim.c) -----
extern "C" {
    pub fn hxml_node_type(n: *const xmlNode) -> c_int;
    pub fn hxml_node_name(n: *const xmlNode) -> *const xmlChar;
    pub fn hxml_node_children(n: *const xmlNode) -> *mut xmlNode;
    pub fn hxml_node_last(n: *const xmlNode) -> *mut xmlNode;
    pub fn hxml_node_parent(n: *const xmlNode) -> *mut xmlNode;
    pub fn hxml_node_next(n: *const xmlNode) -> *mut xmlNode;
    pub fn hxml_node_prev(n: *const xmlNode) -> *mut xmlNode;
    pub fn hxml_node_doc(n: *const xmlNode) -> *mut xmlDoc;
    pub fn hxml_node_properties(n: *const xmlNode) -> *mut xmlAttr;
    pub fn hxml_node_ns(n: *const xmlNode) -> *mut xmlNs;
    pub fn hxml_node_ns_def(n: *const xmlNode) -> *mut xmlNs;
    pub fn hxml_node_raw_content(n: *const xmlNode) -> *const xmlChar;
    pub fn hxml_node_line(n: *const xmlNode) -> c_int;

    pub fn hxml_doc_version(d: *const xmlDoc) -> *const xmlChar;
    pub fn hxml_doc_encoding(d: *const xmlDoc) -> *const xmlChar;
    pub fn hxml_doc_url(d: *const xmlDoc) -> *const xmlChar;
    pub fn hxml_doc_standalone(d: *const xmlDoc) -> c_int;

    pub fn hxml_attr_name(a: *const xmlAttr) -> *const xmlChar;
    pub fn hxml_attr_next(a: *const xmlAttr) -> *mut xmlAttr;
    pub fn hxml_attr_children(a: *const xmlAttr) -> *mut xmlNode;
    pub fn hxml_attr_ns(a: *const xmlAttr) -> *mut xmlNs;

    pub fn hxml_ns_href(n: *const xmlNs) -> *const xmlChar;
    pub fn hxml_ns_prefix(n: *const xmlNs) -> *const xmlChar;
    pub fn hxml_ns_next(n: *const xmlNs) -> *mut xmlNs;

    pub fn hxml_xpath_object_type(o: *const xmlXPathObject) -> c_int;
    pub fn hxml_xpath_object_floatval(o: *const xmlXPathObject) -> c_double;
    pub fn hxml_xpath_object_boolval(o: *const xmlXPathObject) -> c_int;
    pub fn hxml_xpath_object_stringval(o: *const xmlXPathObject) -> *const xmlChar;
    pub fn hxml_xpath_object_nodeset(o: *const xmlXPathObject) -> *mut xmlNodeSet;
    pub fn hxml_nodeset_length(ns: *const xmlNodeSet) -> c_int;
    pub fn hxml_nodeset_item(ns: *const xmlNodeSet, idx: c_int) -> *mut xmlNode;

    pub fn hxml_error_code(e: *const xmlError) -> c_int;
    pub fn hxml_error_level(e: *const xmlError) -> c_int;
    pub fn hxml_error_line(e: *const xmlError) -> c_int;
    pub fn hxml_error_int1(e: *const xmlError) -> c_int;
    pub fn hxml_error_int2(e: *const xmlError) -> c_int;
    pub fn hxml_error_message(e: *const xmlError) -> *const c_char;
    pub fn hxml_error_file(e: *const xmlError) -> *const c_char;

    pub fn hxml_buffer_length(b: *const xmlBuffer) -> c_int;
    pub fn hxml_buffer_content(b: *const xmlBuffer) -> *const xmlChar;

    pub fn hxml_save_doc_to_memory(
        doc: *mut xmlDoc,
        out_buf: *mut *mut xmlChar,
        out_size: *mut c_int,
        encoding: *const c_char,
        format: c_int,
    ) -> c_int;
}

// ============================================================================
// Convenience helpers
// ============================================================================

/// Read a NUL-terminated `xmlChar*` string into a Rust `Vec<u8>`.  Returns an
/// empty vec when the pointer is null.
///
/// # Safety
/// `ptr` must point to a NUL-terminated `xmlChar` string or be null.
pub unsafe fn read_xml_str(ptr: *const xmlChar) -> Vec<u8> {
    if ptr.is_null() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut p = ptr;
    while *p != 0 {
        out.push(*p);
        p = p.add(1);
    }
    out
}

/// Run `hxml_init` once per process so libxml2 is set up before the first call.
pub fn init_once() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        hxml_init();
    });
}

// Silence dead-code lint for crates that don't use the constants.
#[allow(dead_code)]
fn _ensure_constants_in_scope() {
    let _ = element_type::XML_ELEMENT_NODE;
    let _ = xpath_object_type::XPATH_NODESET;
    let _ = reader_type::XML_READER_TYPE_ELEMENT;
    let _ = parser_option::XML_PARSE_NOENT;
    let _ = html_parser_option::HTML_PARSE_RECOVER;
    let _: c_uint = 0;
}
