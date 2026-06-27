/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
/*
 * Thin C shim that exposes safe accessors for the most commonly used
 * libxml2 struct fields.  Doing the field access in C means we don't
 * have to keep the libxml2 struct layouts in sync inside Rust.
 *
 * Naming convention: every function is prefixed `hxml_` (host-libxml)
 * to avoid collisions with libxml2's own symbols.
 */
#define LIBXML_STATIC 1

#include <libxml/HTMLparser.h>
#include <libxml/HTMLtree.h>
#include <libxml/SAX2.h>
#include <libxml/catalog.h>
#include <libxml/encoding.h>
#include <libxml/entities.h>
#include <libxml/globals.h>
#include <libxml/parser.h>
#include <libxml/parserInternals.h>
#include <libxml/relaxng.h>
#include <libxml/tree.h>
#include <libxml/uri.h>
#include <libxml/valid.h>
#include <libxml/xinclude.h>
#include <libxml/xmlIO.h>
#include <libxml/xmlerror.h>
#include <libxml/xmlmemory.h>
#include <libxml/xmlreader.h>
#include <libxml/xmlsave.h>
#include <libxml/xmlschemas.h>
#include <libxml/xmlstring.h>
#include <libxml/xmlversion.h>
#include <libxml/xmlwriter.h>
#include <libxml/xpath.h>
#include <libxml/xpathInternals.h>
#include <libxml/xpointer.h>

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* ---------- One-time initialisation ---------- */

#include <stdarg.h>

/*
 * No-op replacement for the default xmlGenericErrorFunc.  libxml2 calls
 * this with printf-style variadic arguments; we drop them so the touchHLE
 * console doesn't get spammed with parse errors that the guest can still
 * retrieve via xmlGetLastError().
 */
void hxml_no_op_generic_error(void *ctx, const char *msg, ...) {
    (void)ctx;
    (void)msg;
    /* Consume any variadic arguments so we behave like real libxml2 callees. */
    va_list ap;
    va_start(ap, msg);
    va_end(ap);
}

void hxml_no_op_structured_error(void *ctx, const xmlError *err) {
    (void)ctx;
    (void)err;
}

void hxml_install_quiet_error_handlers(void) {
    xmlSetGenericErrorFunc(NULL, hxml_no_op_generic_error);
    xmlSetStructuredErrorFunc(NULL, hxml_no_op_structured_error);
}

void hxml_init(void) {
    /* xmlInitParser is thread-safe and idempotent. */
    xmlInitParser();
    /* Default: tree output is indented but no xmlIndentString shenanigans. */
    xmlIndentTreeOutput = 1;
    xmlKeepBlanksDefault(0);
}

void hxml_cleanup(void) {
    xmlCleanupParser();
}

/* ---------- Generic node accessors ---------- */

int hxml_node_type(const xmlNode *n) {
    return n ? (int)n->type : 0;
}

const xmlChar *hxml_node_name(const xmlNode *n) {
    return n ? n->name : NULL;
}

xmlNodePtr hxml_node_children(const xmlNode *n) {
    return n ? n->children : NULL;
}

xmlNodePtr hxml_node_last(const xmlNode *n) {
    return n ? n->last : NULL;
}

xmlNodePtr hxml_node_parent(const xmlNode *n) {
    return n ? n->parent : NULL;
}

xmlNodePtr hxml_node_next(const xmlNode *n) {
    return n ? n->next : NULL;
}

xmlNodePtr hxml_node_prev(const xmlNode *n) {
    return n ? n->prev : NULL;
}

xmlDocPtr hxml_node_doc(const xmlNode *n) {
    return n ? n->doc : NULL;
}

xmlAttrPtr hxml_node_properties(const xmlNode *n) {
    return (n && (n->type == XML_ELEMENT_NODE)) ? n->properties : NULL;
}

xmlNsPtr hxml_node_ns(const xmlNode *n) {
    if (!n) return NULL;
    if (n->type == XML_ELEMENT_NODE || n->type == XML_ATTRIBUTE_NODE) {
        return n->ns;
    }
    return NULL;
}

xmlNsPtr hxml_node_ns_def(const xmlNode *n) {
    return (n && n->type == XML_ELEMENT_NODE) ? n->nsDef : NULL;
}

const xmlChar *hxml_node_raw_content(const xmlNode *n) {
    /* Direct content pointer.  For text/cdata nodes this is the actual
       payload; for element nodes prefer xmlNodeGetContent which walks
       descendants. */
    return n ? n->content : NULL;
}

int hxml_node_line(const xmlNode *n) {
    return n ? (int)n->line : 0;
}

/* ---------- Doc accessors ---------- */

const xmlChar *hxml_doc_version(const xmlDoc *d) {
    return d ? d->version : NULL;
}

const xmlChar *hxml_doc_encoding(const xmlDoc *d) {
    return d ? d->encoding : NULL;
}

const xmlChar *hxml_doc_url(const xmlDoc *d) {
    return d ? d->URL : NULL;
}

int hxml_doc_standalone(const xmlDoc *d) {
    return d ? d->standalone : 0;
}

/* ---------- Attr accessors ---------- */

const xmlChar *hxml_attr_name(const xmlAttr *a) {
    return a ? a->name : NULL;
}

xmlAttrPtr hxml_attr_next(const xmlAttr *a) {
    return a ? a->next : NULL;
}

xmlNodePtr hxml_attr_children(const xmlAttr *a) {
    return a ? a->children : NULL;
}

xmlNsPtr hxml_attr_ns(const xmlAttr *a) {
    return a ? a->ns : NULL;
}

/* ---------- Ns accessors ---------- */

const xmlChar *hxml_ns_href(const xmlNs *n) {
    return n ? n->href : NULL;
}

const xmlChar *hxml_ns_prefix(const xmlNs *n) {
    return n ? n->prefix : NULL;
}

xmlNsPtr hxml_ns_next(const xmlNs *n) {
    return n ? n->next : NULL;
}

/* ---------- XPath object accessors ---------- */

int hxml_xpath_object_type(const xmlXPathObject *o) {
    return o ? (int)o->type : 0;
}

double hxml_xpath_object_floatval(const xmlXPathObject *o) {
    return o ? o->floatval : 0.0;
}

int hxml_xpath_object_boolval(const xmlXPathObject *o) {
    return o ? o->boolval : 0;
}

const xmlChar *hxml_xpath_object_stringval(const xmlXPathObject *o) {
    return o ? o->stringval : NULL;
}

xmlNodeSetPtr hxml_xpath_object_nodeset(const xmlXPathObject *o) {
    return o ? o->nodesetval : NULL;
}

int hxml_nodeset_length(const xmlNodeSet *ns) {
    return ns ? ns->nodeNr : 0;
}

xmlNodePtr hxml_nodeset_item(const xmlNodeSet *ns, int idx) {
    if (!ns || idx < 0 || idx >= ns->nodeNr) return NULL;
    return ns->nodeTab[idx];
}

/* ---------- Error accessors ---------- */

int hxml_error_code(const xmlError *e) {
    return e ? e->code : 0;
}

int hxml_error_level(const xmlError *e) {
    return e ? (int)e->level : 0;
}

int hxml_error_line(const xmlError *e) {
    return e ? e->line : 0;
}

int hxml_error_int1(const xmlError *e) {
    return e ? e->int1 : 0;
}

int hxml_error_int2(const xmlError *e) {
    return e ? e->int2 : 0;
}

const char *hxml_error_message(const xmlError *e) {
    return e ? e->message : NULL;
}

const char *hxml_error_file(const xmlError *e) {
    return e ? e->file : NULL;
}

/* ---------- TextReader accessors that need careful lifetime handling ----- */

/*
 * xmlTextReaderConstName / ConstValue return pointers owned by the reader; we
 * don't need a wrapper for them.  The wrappers here exist purely to keep the
 * Rust bindings consistent and ensure the layout doesn't drift across versions.
 */

/* ---------- A tiny helper to free xmlChar* allocations safely. ---------- */
void hxml_free(void *ptr) {
    if (ptr) xmlFree(ptr);
}

/* ---------- xmlBuffer helpers ---------- */
int hxml_buffer_length(const xmlBuffer *b) {
    return b ? (int)xmlBufferLength(b) : 0;
}

const xmlChar *hxml_buffer_content(const xmlBuffer *b) {
    return b ? xmlBufferContent(b) : NULL;
}

/* ---------- xmlSaveCtxt convenience ---------- */
int hxml_save_doc_to_memory(xmlDocPtr doc, xmlChar **out_buf, int *out_size,
                            const char *encoding, int format) {
    if (!doc || !out_buf || !out_size) return -1;
    if (format) {
        xmlDocDumpFormatMemoryEnc(doc, out_buf, out_size, encoding, 1);
    } else {
        xmlDocDumpMemoryEnc(doc, out_buf, out_size, encoding);
    }
    return (*out_buf != NULL) ? *out_size : -1;
}
