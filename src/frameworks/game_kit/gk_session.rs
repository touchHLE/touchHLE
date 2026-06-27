/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `GKSession` and related peer-to-peer classes.

use crate::objc::{id, nil, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation GKSession: NSObject

- (id)initWithSessionID:(id)_sessionID // NSString*
           displayName:(id)_displayName // NSString*
           sessionMode:(i32)_sessionMode {
    this
}

- (bool)isAvailable {
    false
}

- (())setAvailable:(bool)_available {
}

- (id)delegate {
    nil
}

- (())setDelegate:(id)_delegate {
}

- (id)sessionID {
    nil
}

- (id)displayName {
    nil
}

- (id)peerID {
    nil
}

- (i32)sessionMode {
    0
}

- (())setDataReceiveHandler:(id)_handler
               withContext:(id)_context {
}

- (())connectToPeer:(id)_peerID
        withTimeout:(f64)_timeout {
}

- (())cancelConnectToPeer:(id)_peerID {
}

- (bool)acceptConnectionFromPeer:(id)_peerID
                           error:(id)_error {
    false
}

- (())denyConnectionFromPeer:(id)_peerID {
}

- (())disconnectFromAllPeers {
}

- (())disconnectPeerFromAllPeers:(id)_peerID {
}

- (id)peersWithConnectionState:(i32)_state {
    nil
}

- (id)displayNameForPeer:(id)_peerID {
    nil
}

- (())sendData:(id)_data
       toPeers:(id)_peers
  withDataMode:(i32)_mode
         error:(id)_error {
}

- (())sendDataToAllPeers:(id)_data
           withDataMode:(i32)_mode
                  error:(id)_error {
}

@end

@implementation GKPeerPickerController: NSObject
// TODO
@end

@implementation GKSessionDelegate: NSObject
// TODO
@end

};
