/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MFMailComposeViewController`, `MFMailComposeViewControllerDelegate`,
//! `MFMessageComposeViewController`, and
//! `MFMessageComposeViewControllerDelegate`.

use crate::frameworks::foundation::ns_string;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

// =========================================================================
// MARK: - MFMailComposeResult constants
// =========================================================================

pub type MFMailComposeResult = i32;
pub const MF_MAIL_COMPOSE_RESULT_CANCELLED: MFMailComposeResult = 0;
pub const MF_MAIL_COMPOSE_RESULT_SAVED: MFMailComposeResult = 1;
pub const MF_MAIL_COMPOSE_RESULT_SENT: MFMailComposeResult = 2;
pub const MF_MAIL_COMPOSE_RESULT_FAILED: MFMailComposeResult = 3;

// =========================================================================
// MARK: - MFMessageComposeResult constants
// =========================================================================

pub type MFMessageComposeResult = i32;
pub const MF_MESSAGE_COMPOSE_RESULT_CANCELLED: MFMessageComposeResult = 0;
pub const MF_MESSAGE_COMPOSE_RESULT_SENT: MFMessageComposeResult = 1;
pub const MF_MESSAGE_COMPOSE_RESULT_FAILED: MFMessageComposeResult = 2;

// =========================================================================
// MARK: - MFMailComposeViewController host object
// =========================================================================

#[derive(Default)]
struct MFMailComposeViewControllerHostObject {
    mail_compose_delegate: id,
    subject: id,
    to_recipients: id,
    cc_recipients: id,
    bcc_recipients: id,
    body: id,
    body_is_html: bool,
    /// NSArray* of NSDictionary* — preferred sending account (iOS 11+)
    preferred_send_account: id,
}
impl HostObject for MFMailComposeViewControllerHostObject {}

// =========================================================================
// MARK: - MFMessageComposeViewController host object
// =========================================================================

#[derive(Default)]
struct MFMessageComposeViewControllerHostObject {
    message_compose_delegate: id,
    /// NSArray* of NSString* recipients (phone numbers / email addresses)
    recipients: id,
    /// NSString* message body
    body: id,
    /// NSString* subject (MMS)
    subject: id,
    /// Whether to disable user body editing (iOS 7+)
    disables_user_attachments: bool,
}
impl HostObject for MFMessageComposeViewControllerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - MFMailComposeViewController
// =========================================================================

@implementation MFMailComposeViewController: UINavigationController

+ (bool)canSendMail {
    log_dbg!("MFMailComposeViewController canSendMail — returning NO");
    false
}

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(MFMailComposeViewControllerHostObject {
        mail_compose_delegate: nil,
        subject: nil,
        to_recipients: nil,
        cc_recipients: nil,
        bcc_recipients: nil,
        body: nil,
        body_is_html: false,
        preferred_send_account: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init { this }

- (())dealloc {
    let host = env.objc.borrow::<MFMailComposeViewControllerHostObject>(this);
    let (delegate, subject, to, cc, bcc, body, psa) = (
        host.mail_compose_delegate,
        host.subject,
        host.to_recipients,
        host.cc_recipients,
        host.bcc_recipients,
        host.body,
        host.preferred_send_account,
    );
    release(env, delegate);
    release(env, subject);
    release(env, to);
    release(env, cc);
    release(env, bcc);
    release(env, body);
    release(env, psa);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Delegate

- (id)mailComposeDelegate {
    env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).mail_compose_delegate
}

- (())setMailComposeDelegate:(id)delegate {
    let old = env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).mail_compose_delegate;
    release(env, old);
    retain(env, delegate);
    env.objc.borrow_mut::<MFMailComposeViewControllerHostObject>(this)
        .mail_compose_delegate = delegate;
}

// MARK: Subject

- (id)subject {
    env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).subject
}

- (())setSubject:(id)subject {
    let old = env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).subject;
    release(env, old);
    retain(env, subject);
    env.objc.borrow_mut::<MFMailComposeViewControllerHostObject>(this).subject = subject;
}

// MARK: Recipients

- (id)toRecipients {
    env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).to_recipients
}

- (())setToRecipients:(id)recipients {
    let old = env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).to_recipients;
    release(env, old);
    retain(env, recipients);
    env.objc.borrow_mut::<MFMailComposeViewControllerHostObject>(this).to_recipients = recipients;
}

- (id)ccRecipients {
    env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).cc_recipients
}

- (())setCcRecipients:(id)recipients {
    let old = env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).cc_recipients;
    release(env, old);
    retain(env, recipients);
    env.objc.borrow_mut::<MFMailComposeViewControllerHostObject>(this).cc_recipients = recipients;
}

- (id)bccRecipients {
    env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).bcc_recipients
}

- (())setBccRecipients:(id)recipients {
    let old = env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).bcc_recipients;
    release(env, old);
    retain(env, recipients);
    env.objc.borrow_mut::<MFMailComposeViewControllerHostObject>(this).bcc_recipients = recipients;
}

// MARK: Body

- (id)messageBody {
    env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).body
}

- (bool)isBodyHTML {
    env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).body_is_html
}

- (())setMessageBody:(id)body isHTML:(bool)is_html {
    let old = env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).body;
    release(env, old);
    retain(env, body);
    let host = env.objc.borrow_mut::<MFMailComposeViewControllerHostObject>(this);
    host.body = body;
    host.body_is_html = is_html;
}

// MARK: Preferred send account (iOS 11+, stub)

- (id)preferredSendingEmailAddress {
    env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).preferred_send_account
}

- (())setPreferredSendingEmailAddress:(id)address {
    let old = env.objc.borrow::<MFMailComposeViewControllerHostObject>(this).preferred_send_account;
    release(env, old);
    retain(env, address);
    env.objc.borrow_mut::<MFMailComposeViewControllerHostObject>(this)
        .preferred_send_account = address;
}

// MARK: Attachments

- (())addAttachmentData:(id)_attachment
              mimeType:(id)mime_type
              fileName:(id)file_name {
    let mime = if mime_type != nil {
        ns_string::to_rust_string(env, mime_type).into_owned()
    } else { "(null)".into() };
    let name = if file_name != nil {
        ns_string::to_rust_string(env, file_name).into_owned()
    } else { "(null)".into() };
    log!(
        "MFMailComposeViewController addAttachmentData mimeType:{} fileName:{} — ignored",
        mime, name
    );
}

// MARK: Presentation

- (())viewWillAppear:(bool)_animated {
    let delegate = env.objc.borrow::<MFMailComposeViewControllerHostObject>(this)
        .mail_compose_delegate;
    if delegate != nil {
        let sel = env.objc
            .lookup_selector("mailComposeController:didFinishWithResult:error:")
            .unwrap();
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let error: id = nil;
            let _: () = msg![env; delegate mailComposeController:this
                                             didFinishWithResult:MF_MAIL_COMPOSE_RESULT_CANCELLED
                                                           error:error];
        }
    }
}

- (())viewDidAppear:(bool)animated {
    let presenting: id = msg![env; this presentingViewController];
    if presenting != nil {
        let _: () = msg![env; presenting dismissViewControllerAnimated:animated completion:nil];
    }
}

// MARK: Description

- (id)description {
    let (subject, body_is_html) = {
        let h = env.objc.borrow::<MFMailComposeViewControllerHostObject>(this);
        (h.subject, h.body_is_html)
    };
    let subject_str = if subject != nil {
        ns_string::to_rust_string(env, subject).into_owned()
    } else { "(null)".into() };
    let s = format!(
        "<MFMailComposeViewController: subject=\"{}\" bodyIsHTML={}>",
        subject_str, body_is_html
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

// =========================================================================
// MARK: - MFMessageComposeViewController
// =========================================================================

@implementation MFMessageComposeViewController: UINavigationController

// MARK: Availability

+ (bool)canSendText {
    log_dbg!("MFMessageComposeViewController canSendText — returning NO");
    false
}

+ (bool)canSendSubject {
    false
}

+ (bool)canSendAttachments {
    false
}

+ (bool)isSupportedAttachmentUTI:(id)_uti {
    false
}

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(MFMessageComposeViewControllerHostObject {
        message_compose_delegate: nil,
        recipients: nil,
        body: nil,
        subject: nil,
        disables_user_attachments: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init { this }

- (())dealloc {
    let host = env.objc.borrow::<MFMessageComposeViewControllerHostObject>(this);
    let (delegate, recipients, body, subject) = (
        host.message_compose_delegate,
        host.recipients,
        host.body,
        host.subject,
    );
    release(env, delegate);
    release(env, recipients);
    release(env, body);
    release(env, subject);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Delegate

- (id)messageComposeDelegate {
    env.objc.borrow::<MFMessageComposeViewControllerHostObject>(this).message_compose_delegate
}

- (())setMessageComposeDelegate:(id)delegate {
    let old = env.objc.borrow::<MFMessageComposeViewControllerHostObject>(this)
        .message_compose_delegate;
    release(env, old);
    retain(env, delegate);
    env.objc.borrow_mut::<MFMessageComposeViewControllerHostObject>(this)
        .message_compose_delegate = delegate;
}

// MARK: Recipients

- (id)recipients { // NSArray<NSString*>*
    env.objc.borrow::<MFMessageComposeViewControllerHostObject>(this).recipients
}

- (())setRecipients:(id)recipients {
    let old = env.objc.borrow::<MFMessageComposeViewControllerHostObject>(this).recipients;
    release(env, old);
    retain(env, recipients);
    env.objc.borrow_mut::<MFMessageComposeViewControllerHostObject>(this).recipients = recipients;
}

// MARK: Body

- (id)body { // NSString*
    env.objc.borrow::<MFMessageComposeViewControllerHostObject>(this).body
}

- (())setBody:(id)body {
    let old = env.objc.borrow::<MFMessageComposeViewControllerHostObject>(this).body;
    release(env, old);
    retain(env, body);
    env.objc.borrow_mut::<MFMessageComposeViewControllerHostObject>(this).body = body;
}

// MARK: Subject (MMS)

- (id)subject {
    env.objc.borrow::<MFMessageComposeViewControllerHostObject>(this).subject
}

- (())setSubject:(id)subject {
    let old = env.objc.borrow::<MFMessageComposeViewControllerHostObject>(this).subject;
    release(env, old);
    retain(env, subject);
    env.objc.borrow_mut::<MFMessageComposeViewControllerHostObject>(this).subject = subject;
}

// MARK: Attachments (stubs)

- (bool)addAttachmentURL:(id)_url withAlternateFilename:(id)_filename {
    log!("MFMessageComposeViewController addAttachmentURL: — ignored (no SMS support)");
    false
}

- (bool)addAttachmentData:(id)_data
              typeIdentifier:(id)_uti
                 filename:(id)filename {
    let name = if filename != nil {
        ns_string::to_rust_string(env, filename).into_owned()
    } else { "(null)".into() };
    log!("MFMessageComposeViewController addAttachmentData filename:{} — ignored", name);
    false
}

- (())disableUserAttachments {
    env.objc.borrow_mut::<MFMessageComposeViewControllerHostObject>(this)
        .disables_user_attachments = true;
}

- (id)attachments { // NSArray* — always empty
    msg_class![env; NSArray array]
}

// MARK: Presentation
// No SMS UI in touchHLE. Immediately fire cancelled delegate callback.

- (())viewWillAppear:(bool)_animated {
    let delegate = env.objc.borrow::<MFMessageComposeViewControllerHostObject>(this)
        .message_compose_delegate;
    if delegate != nil {
        let sel = env.objc
            .lookup_selector("messageComposeViewController:didFinishWithResult:")
            .unwrap();
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let _: () = msg![env; delegate messageComposeViewController:this
                                              didFinishWithResult:MF_MESSAGE_COMPOSE_RESULT_CANCELLED];
        }
    }
}

- (())viewDidAppear:(bool)animated {
    let presenting: id = msg![env; this presentingViewController];
    if presenting != nil {
        let _: () = msg![env; presenting dismissViewControllerAnimated:animated completion:nil];
    }
}

// MARK: Description

- (id)description {
    let (body, recipients_count) = {
        let h = env.objc.borrow::<MFMessageComposeViewControllerHostObject>(this);
        let r = h.recipients;
        let body = h.body;
        let count: crate::frameworks::foundation::NSUInteger = if r != nil {
            msg![env; r count]
        } else { 0 };
        (body, count)
    };
    let body_str = if body != nil {
        ns_string::to_rust_string(env, body).into_owned()
    } else { "(null)".into() };
    let s = format!(
        "<MFMessageComposeViewController: recipients={} body=\"{}\">",
        recipients_count, body_str
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};
