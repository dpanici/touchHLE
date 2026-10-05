/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMediaPlaylist`.

use crate::dyld::{ConstantExports, HostConstant};
use crate::objc::{objc_classes, ClassExports};

pub const MPMediaPlaylistPropertyName: &str = "name";

/// `NSString*` keys for `valueForProperty:`.
pub const CONSTANTS: ConstantExports = &[(
    "_MPMediaPlaylistPropertyName",
    HostConstant::NSString(MPMediaPlaylistPropertyName),
)];

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMediaPlaylist: MPMediaItemCollection
// TODO
@end

};
