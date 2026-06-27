/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMusicPlayerController` etc.

use crate::{
    dyld::{ConstantExports, HostConstant},
    objc::{id, nil, objc_classes, ClassExports},
};

pub const MPMusicPlayerControllerNowPlayingItemDidChangeNotification: &str =
    "MPMusicPlayerControllerNowPlayingItemDidChangeNotification";
pub const MPMusicPlayerControllerPlaybackStateDidChangeNotification: &str =
    "MPMusicPlayerControllerPlaybackStateDidChangeNotification";
pub const MPMusicPlayerControllerVolumeDidChangeNotification: &str =
    "MPMusicPlayerControllerVolumeDidChangeNotification";

// `MPMediaItem` property keys. Per Apple's `MPMediaItem.h` headers, each of
// these is an `NSString *` constant exported as a non-lazy symbol; the
// literal value is the documented Cocoa property name. Earlier iOS versions
// (3.x) only define a subset, but we include the full set so apps built
// against newer SDKs that still target old runtimes resolve the symbol.
pub const MPMediaItemPropertyPersistentID: &str = "persistentID";
pub const MPMediaItemPropertyAlbumPersistentID: &str = "albumPersistentID";
pub const MPMediaItemPropertyArtistPersistentID: &str = "artistPersistentID";
pub const MPMediaItemPropertyAlbumArtistPersistentID: &str = "albumArtistPersistentID";
pub const MPMediaItemPropertyGenrePersistentID: &str = "genrePersistentID";
pub const MPMediaItemPropertyComposerPersistentID: &str = "composerPersistentID";
pub const MPMediaItemPropertyPodcastPersistentID: &str = "podcastPersistentID";
pub const MPMediaItemPropertyMediaType: &str = "mediaType";
pub const MPMediaItemPropertyTitle: &str = "title";
pub const MPMediaItemPropertyAlbumTitle: &str = "albumTitle";
pub const MPMediaItemPropertyArtist: &str = "artist";
pub const MPMediaItemPropertyAlbumArtist: &str = "albumArtist";
pub const MPMediaItemPropertyGenre: &str = "genre";
pub const MPMediaItemPropertyComposer: &str = "composer";
pub const MPMediaItemPropertyPlaybackDuration: &str = "playbackDuration";
pub const MPMediaItemPropertyAlbumTrackNumber: &str = "albumTrackNumber";
pub const MPMediaItemPropertyAlbumTrackCount: &str = "albumTrackCount";
pub const MPMediaItemPropertyDiscNumber: &str = "discNumber";
pub const MPMediaItemPropertyDiscCount: &str = "discCount";
pub const MPMediaItemPropertyArtwork: &str = "artwork";
pub const MPMediaItemPropertyLyrics: &str = "lyrics";
pub const MPMediaItemPropertyIsCompilation: &str = "isCompilation";
pub const MPMediaItemPropertyReleaseDate: &str = "releaseDate";
pub const MPMediaItemPropertyBeatsPerMinute: &str = "beatsPerMinute";
pub const MPMediaItemPropertyComments: &str = "comments";
pub const MPMediaItemPropertyAssetURL: &str = "assetURL";
pub const MPMediaItemPropertyIsCloudItem: &str = "isCloudItem";
pub const MPMediaItemPropertyHasProtectedAsset: &str = "hasProtectedAsset";
pub const MPMediaItemPropertyPodcastTitle: &str = "podcastTitle";
pub const MPMediaItemPropertyPlayCount: &str = "playCount";
pub const MPMediaItemPropertySkipCount: &str = "skipCount";
pub const MPMediaItemPropertyRating: &str = "rating";
pub const MPMediaItemPropertyLastPlayedDate: &str = "lastPlayedDate";
pub const MPMediaItemPropertyUserGrouping: &str = "userGrouping";
pub const MPMediaItemPropertyBookmarkTime: &str = "bookmarkTime";
pub const MPMediaItemPropertyIsExplicit: &str = "isExplicit";
pub const MPMediaItemPropertyDateAdded: &str = "dateAdded";
pub const MPMediaItemPropertyPlaybackStoreID: &str = "playbackStoreID";

/// `NSNotificationName` and `MPMediaItemProperty*` values.
pub const CONSTANTS: ConstantExports = &[
    (
        "_MPMusicPlayerControllerNowPlayingItemDidChangeNotification",
        HostConstant::NSString(MPMusicPlayerControllerNowPlayingItemDidChangeNotification),
    ),
    (
        "_MPMusicPlayerControllerPlaybackStateDidChangeNotification",
        HostConstant::NSString(MPMusicPlayerControllerPlaybackStateDidChangeNotification),
    ),
    (
        "_MPMusicPlayerControllerVolumeDidChangeNotification",
        HostConstant::NSString(MPMusicPlayerControllerVolumeDidChangeNotification),
    ),
    (
        "_MPMediaItemPropertyPersistentID",
        HostConstant::NSString(MPMediaItemPropertyPersistentID),
    ),
    (
        "_MPMediaItemPropertyAlbumPersistentID",
        HostConstant::NSString(MPMediaItemPropertyAlbumPersistentID),
    ),
    (
        "_MPMediaItemPropertyArtistPersistentID",
        HostConstant::NSString(MPMediaItemPropertyArtistPersistentID),
    ),
    (
        "_MPMediaItemPropertyAlbumArtistPersistentID",
        HostConstant::NSString(MPMediaItemPropertyAlbumArtistPersistentID),
    ),
    (
        "_MPMediaItemPropertyGenrePersistentID",
        HostConstant::NSString(MPMediaItemPropertyGenrePersistentID),
    ),
    (
        "_MPMediaItemPropertyComposerPersistentID",
        HostConstant::NSString(MPMediaItemPropertyComposerPersistentID),
    ),
    (
        "_MPMediaItemPropertyPodcastPersistentID",
        HostConstant::NSString(MPMediaItemPropertyPodcastPersistentID),
    ),
    (
        "_MPMediaItemPropertyMediaType",
        HostConstant::NSString(MPMediaItemPropertyMediaType),
    ),
    (
        "_MPMediaItemPropertyTitle",
        HostConstant::NSString(MPMediaItemPropertyTitle),
    ),
    (
        "_MPMediaItemPropertyAlbumTitle",
        HostConstant::NSString(MPMediaItemPropertyAlbumTitle),
    ),
    (
        "_MPMediaItemPropertyArtist",
        HostConstant::NSString(MPMediaItemPropertyArtist),
    ),
    (
        "_MPMediaItemPropertyAlbumArtist",
        HostConstant::NSString(MPMediaItemPropertyAlbumArtist),
    ),
    (
        "_MPMediaItemPropertyGenre",
        HostConstant::NSString(MPMediaItemPropertyGenre),
    ),
    (
        "_MPMediaItemPropertyComposer",
        HostConstant::NSString(MPMediaItemPropertyComposer),
    ),
    (
        "_MPMediaItemPropertyPlaybackDuration",
        HostConstant::NSString(MPMediaItemPropertyPlaybackDuration),
    ),
    (
        "_MPMediaItemPropertyAlbumTrackNumber",
        HostConstant::NSString(MPMediaItemPropertyAlbumTrackNumber),
    ),
    (
        "_MPMediaItemPropertyAlbumTrackCount",
        HostConstant::NSString(MPMediaItemPropertyAlbumTrackCount),
    ),
    (
        "_MPMediaItemPropertyDiscNumber",
        HostConstant::NSString(MPMediaItemPropertyDiscNumber),
    ),
    (
        "_MPMediaItemPropertyDiscCount",
        HostConstant::NSString(MPMediaItemPropertyDiscCount),
    ),
    (
        "_MPMediaItemPropertyArtwork",
        HostConstant::NSString(MPMediaItemPropertyArtwork),
    ),
    (
        "_MPMediaItemPropertyLyrics",
        HostConstant::NSString(MPMediaItemPropertyLyrics),
    ),
    (
        "_MPMediaItemPropertyIsCompilation",
        HostConstant::NSString(MPMediaItemPropertyIsCompilation),
    ),
    (
        "_MPMediaItemPropertyReleaseDate",
        HostConstant::NSString(MPMediaItemPropertyReleaseDate),
    ),
    (
        "_MPMediaItemPropertyBeatsPerMinute",
        HostConstant::NSString(MPMediaItemPropertyBeatsPerMinute),
    ),
    (
        "_MPMediaItemPropertyComments",
        HostConstant::NSString(MPMediaItemPropertyComments),
    ),
    (
        "_MPMediaItemPropertyAssetURL",
        HostConstant::NSString(MPMediaItemPropertyAssetURL),
    ),
    (
        "_MPMediaItemPropertyIsCloudItem",
        HostConstant::NSString(MPMediaItemPropertyIsCloudItem),
    ),
    (
        "_MPMediaItemPropertyHasProtectedAsset",
        HostConstant::NSString(MPMediaItemPropertyHasProtectedAsset),
    ),
    (
        "_MPMediaItemPropertyPodcastTitle",
        HostConstant::NSString(MPMediaItemPropertyPodcastTitle),
    ),
    (
        "_MPMediaItemPropertyPlayCount",
        HostConstant::NSString(MPMediaItemPropertyPlayCount),
    ),
    (
        "_MPMediaItemPropertySkipCount",
        HostConstant::NSString(MPMediaItemPropertySkipCount),
    ),
    (
        "_MPMediaItemPropertyRating",
        HostConstant::NSString(MPMediaItemPropertyRating),
    ),
    (
        "_MPMediaItemPropertyLastPlayedDate",
        HostConstant::NSString(MPMediaItemPropertyLastPlayedDate),
    ),
    (
        "_MPMediaItemPropertyUserGrouping",
        HostConstant::NSString(MPMediaItemPropertyUserGrouping),
    ),
    (
        "_MPMediaItemPropertyBookmarkTime",
        HostConstant::NSString(MPMediaItemPropertyBookmarkTime),
    ),
    (
        "_MPMediaItemPropertyIsExplicit",
        HostConstant::NSString(MPMediaItemPropertyIsExplicit),
    ),
    (
        "_MPMediaItemPropertyDateAdded",
        HostConstant::NSString(MPMediaItemPropertyDateAdded),
    ),
    (
        "_MPMediaItemPropertyPlaybackStoreID",
        HostConstant::NSString(MPMediaItemPropertyPlaybackStoreID),
    ),
];

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMusicPlayerController: NSObject

+ (id)iPodMusicPlayer {
    log_dbg!(
        "TODO: [(MPMusicPlayerController*){:?} iPodMusicPlayer]",
        this
    );
    nil
}

+ (id)applicationMusicPlayer {
    log_dbg!(
        "TODO: [(MPMusicPlayerController*){:?} applicationMusicPlayer]",
        this
    );
    nil
}

@end

};
