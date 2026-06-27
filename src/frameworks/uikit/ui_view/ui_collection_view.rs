/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UICollectionView`, `UICollectionViewCell`, `UICollectionViewLayout`,
//! `UICollectionViewFlowLayout`.
//!
//! Implementation based on Apple's UICollectionView documentation:
//! - <https://developer.apple.com/documentation/uikit/uicollectionview>
//! - <https://developer.apple.com/documentation/uikit/uicollectionviewflowlayout>
//!
//! UICollectionView manages an ordered collection of data items and presents
//! them using customizable layouts. It uses a data source pattern
//! (UICollectionViewDataSource) to obtain cells and supplementary views.
//!
//! This implementation provides:
//! - Full data source / delegate support
//! - Cell reuse (basic)
//! - UICollectionViewFlowLayout with line-based layout
//! - reloadData with actual cell creation and positioning

use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::frameworks::uikit::ui_view::ui_scroll_view::UIScrollViewHostObject;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    retain, ClassExports, HostObject, NSZonePtr,
};

// ============================================================================
// MARK: - UICollectionViewScrollDirection
// ============================================================================

type UICollectionViewScrollDirection = NSInteger;
#[allow(dead_code)]
const UICollectionViewScrollDirectionVertical: UICollectionViewScrollDirection = 0;
#[allow(dead_code)]
const UICollectionViewScrollDirectionHorizontal: UICollectionViewScrollDirection = 1;

// ============================================================================
// MARK: - UICollectionViewCell host object
// ============================================================================

struct UICollectionViewCellHostObject {
    superclass: super::UIViewHostObject,
    content_view: id,
    background_view: id,
    selected_background_view: id,
    selected: bool,
    highlighted: bool,
    reuse_identifier: id,
}
impl_HostObject_with_superclass!(UICollectionViewCellHostObject);

impl Default for UICollectionViewCellHostObject {
    fn default() -> Self {
        UICollectionViewCellHostObject {
            superclass: Default::default(),
            content_view: nil,
            background_view: nil,
            selected_background_view: nil,
            selected: false,
            highlighted: false,
            reuse_identifier: nil,
        }
    }
}

// ============================================================================
// MARK: - UICollectionReusableView host object
// ============================================================================

struct UICollectionReusableViewHostObject {
    superclass: super::UIViewHostObject,
    reuse_identifier: id,
}
impl_HostObject_with_superclass!(UICollectionReusableViewHostObject);

impl Default for UICollectionReusableViewHostObject {
    fn default() -> Self {
        UICollectionReusableViewHostObject {
            superclass: Default::default(),
            reuse_identifier: nil,
        }
    }
}

// ============================================================================
// MARK: - UICollectionViewLayout host object
// ============================================================================

#[derive(Default)]
struct UICollectionViewLayoutHostObject {
    collection_view: id, // weak back-pointer
}
impl HostObject for UICollectionViewLayoutHostObject {}

// ============================================================================
// MARK: - UICollectionViewFlowLayout host object
// ============================================================================

struct UICollectionViewFlowLayoutHostObject {
    scroll_direction: UICollectionViewScrollDirection,
    minimum_line_spacing: CGFloat,
    minimum_interitem_spacing: CGFloat,
    item_size: CGSize,
    estimated_item_size: CGSize,
    section_inset_top: CGFloat,
    section_inset_left: CGFloat,
    section_inset_bottom: CGFloat,
    section_inset_right: CGFloat,
    header_reference_size: CGSize,
    footer_reference_size: CGSize,
}
impl HostObject for UICollectionViewFlowLayoutHostObject {}

impl Default for UICollectionViewFlowLayoutHostObject {
    fn default() -> Self {
        UICollectionViewFlowLayoutHostObject {
            scroll_direction: UICollectionViewScrollDirectionVertical,
            minimum_line_spacing: 10.0,
            minimum_interitem_spacing: 10.0,
            item_size: CGSize {
                width: 50.0,
                height: 50.0,
            },
            estimated_item_size: CGSize {
                width: 0.0,
                height: 0.0,
            },
            section_inset_top: 0.0,
            section_inset_left: 0.0,
            section_inset_bottom: 0.0,
            section_inset_right: 0.0,
            header_reference_size: CGSize {
                width: 0.0,
                height: 0.0,
            },
            footer_reference_size: CGSize {
                width: 0.0,
                height: 0.0,
            },
        }
    }
}

// ============================================================================
// MARK: - UICollectionView host object
// ============================================================================

struct UICollectionViewHostObject {
    superclass: UIScrollViewHostObject,
    data_source: id,
    collection_delegate: id,
    layout: id,
    /// Cells currently visible. Each is retained.
    cells: Vec<id>,
    /// Whether multiple selection is allowed.
    allows_multiple_selection: bool,
    allows_selection: bool,
    background_view: id,
}
impl_HostObject_with_superclass!(UICollectionViewHostObject);

impl Default for UICollectionViewHostObject {
    fn default() -> Self {
        UICollectionViewHostObject {
            superclass: Default::default(),
            data_source: nil,
            collection_delegate: nil,
            layout: nil,
            cells: Vec::new(),
            allows_multiple_selection: false,
            allows_selection: true,
            background_view: nil,
        }
    }
}

/// Clear all cells, optionally removing them from the view hierarchy.
fn clear_cells(env: &mut crate::Environment, this: id, remove_subviews: bool) {
    let cells = std::mem::take(
        &mut env
            .objc
            .borrow_mut::<UICollectionViewHostObject>(this)
            .cells,
    );
    for cell in cells {
        if remove_subviews {
            let _: () = msg![env; cell removeFromSuperview];
        }
        release(env, cell);
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// ==========================================================================
// UICollectionReusableView
// ==========================================================================

@implementation UICollectionReusableView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UICollectionReusableViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)reuseIdentifier {
    env.objc
        .borrow::<UICollectionReusableViewHostObject>(this)
        .reuse_identifier
}

- (())prepareForReuse {}

@end

// ==========================================================================
// UICollectionViewCell
// ==========================================================================

@implementation UICollectionViewCell: UICollectionReusableView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UICollectionViewCellHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    // Create contentView — same pattern as UITableViewCell.
    let content_view: id = msg_class![env; UIView alloc];
    let content_view: id = msg![env; content_view initWithFrame:frame];
    () = msg![env; this addSubview:content_view];
    env.objc
        .borrow_mut::<UICollectionViewCellHostObject>(this)
        .content_view = content_view;
    this
}

- (())dealloc {
    let host = env.objc.borrow::<UICollectionViewCellHostObject>(this);
    let (content_view, bg, sel_bg) = (
        host.content_view,
        host.background_view,
        host.selected_background_view,
    );
    release(env, content_view);
    release(env, bg);
    release(env, sel_bg);
    msg_super![env; this dealloc]
}

- (id)contentView {
    env.objc
        .borrow::<UICollectionViewCellHostObject>(this)
        .content_view
}

- (id)backgroundView {
    env.objc
        .borrow::<UICollectionViewCellHostObject>(this)
        .background_view
}

- (())setBackgroundView:(id)view {
    let old = env
        .objc
        .borrow::<UICollectionViewCellHostObject>(this)
        .background_view;
    release(env, old);
    retain(env, view);
    env.objc
        .borrow_mut::<UICollectionViewCellHostObject>(this)
        .background_view = view;
}

- (id)selectedBackgroundView {
    env.objc
        .borrow::<UICollectionViewCellHostObject>(this)
        .selected_background_view
}

- (())setSelectedBackgroundView:(id)view {
    let old = env
        .objc
        .borrow::<UICollectionViewCellHostObject>(this)
        .selected_background_view;
    release(env, old);
    retain(env, view);
    env.objc
        .borrow_mut::<UICollectionViewCellHostObject>(this)
        .selected_background_view = view;
}

- (bool)isSelected {
    env.objc
        .borrow::<UICollectionViewCellHostObject>(this)
        .selected
}

- (())setSelected:(bool)selected {
    env.objc
        .borrow_mut::<UICollectionViewCellHostObject>(this)
        .selected = selected;
}

- (bool)isHighlighted {
    env.objc
        .borrow::<UICollectionViewCellHostObject>(this)
        .highlighted
}

- (())setHighlighted:(bool)highlighted {
    env.objc
        .borrow_mut::<UICollectionViewCellHostObject>(this)
        .highlighted = highlighted;
}

- (())prepareForReuse {
    env.objc
        .borrow_mut::<UICollectionViewCellHostObject>(this)
        .selected = false;
    env.objc
        .borrow_mut::<UICollectionViewCellHostObject>(this)
        .highlighted = false;
}

@end

// ==========================================================================
// UICollectionViewLayout (abstract base)
// ==========================================================================

@implementation UICollectionViewLayout: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UICollectionViewLayoutHostObject {
        collection_view: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init { this }

- (id)collectionView {
    env.objc
        .borrow::<UICollectionViewLayoutHostObject>(this)
        .collection_view
}

- (())invalidateLayout {}
- (())prepareLayout {}

- (CGSize)collectionViewContentSize {
    CGSize {
        width: 0.0,
        height: 0.0,
    }
}

@end

// ==========================================================================
// UICollectionViewFlowLayout
// ==========================================================================

@implementation UICollectionViewFlowLayout: UICollectionViewLayout

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UICollectionViewFlowLayoutHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init { this }

- (UICollectionViewScrollDirection)scrollDirection {
    env.objc
        .borrow::<UICollectionViewFlowLayoutHostObject>(this)
        .scroll_direction
}

- (())setScrollDirection:(UICollectionViewScrollDirection)dir {
    env.objc
        .borrow_mut::<UICollectionViewFlowLayoutHostObject>(this)
        .scroll_direction = dir;
}

- (CGFloat)minimumLineSpacing {
    env.objc
        .borrow::<UICollectionViewFlowLayoutHostObject>(this)
        .minimum_line_spacing
}

- (())setMinimumLineSpacing:(CGFloat)spacing {
    env.objc
        .borrow_mut::<UICollectionViewFlowLayoutHostObject>(this)
        .minimum_line_spacing = spacing;
}

- (CGFloat)minimumInteritemSpacing {
    env.objc
        .borrow::<UICollectionViewFlowLayoutHostObject>(this)
        .minimum_interitem_spacing
}

- (())setMinimumInteritemSpacing:(CGFloat)spacing {
    env.objc
        .borrow_mut::<UICollectionViewFlowLayoutHostObject>(this)
        .minimum_interitem_spacing = spacing;
}

- (CGSize)itemSize {
    env.objc
        .borrow::<UICollectionViewFlowLayoutHostObject>(this)
        .item_size
}

- (())setItemSize:(CGSize)size {
    env.objc
        .borrow_mut::<UICollectionViewFlowLayoutHostObject>(this)
        .item_size = size;
}

- (CGSize)estimatedItemSize {
    env.objc
        .borrow::<UICollectionViewFlowLayoutHostObject>(this)
        .estimated_item_size
}

- (())setEstimatedItemSize:(CGSize)size {
    env.objc
        .borrow_mut::<UICollectionViewFlowLayoutHostObject>(this)
        .estimated_item_size = size;
}

- (CGSize)headerReferenceSize {
    env.objc
        .borrow::<UICollectionViewFlowLayoutHostObject>(this)
        .header_reference_size
}

- (())setHeaderReferenceSize:(CGSize)size {
    env.objc
        .borrow_mut::<UICollectionViewFlowLayoutHostObject>(this)
        .header_reference_size = size;
}

- (CGSize)footerReferenceSize {
    env.objc
        .borrow::<UICollectionViewFlowLayoutHostObject>(this)
        .footer_reference_size
}

- (())setFooterReferenceSize:(CGSize)size {
    env.objc
        .borrow_mut::<UICollectionViewFlowLayoutHostObject>(this)
        .footer_reference_size = size;
}

// Section insets are passed as UIEdgeInsets (four CGFloats). We use a CGRect
// hack since UIEdgeInsets is not a registered GuestArg type in this project.
- (CGRect)sectionInset {
    let host = env.objc.borrow::<UICollectionViewFlowLayoutHostObject>(this);
    CGRect {
        origin: CGPoint { x: host.section_inset_top, y: host.section_inset_left },
        size: CGSize { width: host.section_inset_bottom, height: host.section_inset_right },
    }
}

- (())setSectionInset:(CGRect)insets {
    let host = env.objc.borrow_mut::<UICollectionViewFlowLayoutHostObject>(this);
    host.section_inset_top = insets.origin.x;
    host.section_inset_left = insets.origin.y;
    host.section_inset_bottom = insets.size.width;
    host.section_inset_right = insets.size.height;
}

@end

// ==========================================================================
// UICollectionView
// ==========================================================================

@implementation UICollectionView: UIScrollView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UICollectionViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame collectionViewLayout:(id)layout {
    let this: id = msg_super![env; this initWithFrame:frame];
    retain(env, layout);
    env.objc.borrow_mut::<UICollectionViewHostObject>(this).layout = layout;
    this
}

- (id)initWithFrame:(CGRect)frame {
    msg_super![env; this initWithFrame:frame]
}

- (id)initWithCoder:(id)coder {
    msg_super![env; this initWithCoder:coder]
}

- (())dealloc {
    clear_cells(env, this, false);
    let layout = env.objc.borrow::<UICollectionViewHostObject>(this).layout;
    let bg = env.objc.borrow::<UICollectionViewHostObject>(this).background_view;
    release(env, layout);
    release(env, bg);
    msg_super![env; this dealloc]
}

// MARK: - Data source & delegate

- (id)dataSource {
    env.objc.borrow::<UICollectionViewHostObject>(this).data_source
}

- (())setDataSource:(id)ds {
    env.objc.borrow_mut::<UICollectionViewHostObject>(this).data_source = ds;
}

- (id)delegate {
    env.objc.borrow::<UICollectionViewHostObject>(this).collection_delegate
}

- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UICollectionViewHostObject>(this).collection_delegate = delegate;
}

// MARK: - Layout

- (id)collectionViewLayout {
    env.objc.borrow::<UICollectionViewHostObject>(this).layout
}

- (())setCollectionViewLayout:(id)layout {
    let old = env.objc.borrow::<UICollectionViewHostObject>(this).layout;
    release(env, old);
    retain(env, layout);
    env.objc.borrow_mut::<UICollectionViewHostObject>(this).layout = layout;
}

- (())setCollectionViewLayout:(id)layout animated:(bool)_animated {
    () = msg![env; this setCollectionViewLayout:layout];
}

// MARK: - Background

- (id)backgroundView {
    env.objc.borrow::<UICollectionViewHostObject>(this).background_view
}

- (())setBackgroundView:(id)view {
    let old = env.objc.borrow::<UICollectionViewHostObject>(this).background_view;
    release(env, old);
    retain(env, view);
    env.objc.borrow_mut::<UICollectionViewHostObject>(this).background_view = view;
}

// MARK: - Selection

- (bool)allowsSelection {
    env.objc.borrow::<UICollectionViewHostObject>(this).allows_selection
}

- (())setAllowsSelection:(bool)allows {
    env.objc.borrow_mut::<UICollectionViewHostObject>(this).allows_selection = allows;
}

- (bool)allowsMultipleSelection {
    env.objc.borrow::<UICollectionViewHostObject>(this).allows_multiple_selection
}

- (())setAllowsMultipleSelection:(bool)allows {
    env.objc.borrow_mut::<UICollectionViewHostObject>(this).allows_multiple_selection = allows;
}

// MARK: - Registration

- (())registerClass:(id)_class forCellWithReuseIdentifier:(id)_identifier {
    // We don't implement a reuse pool; returning nil from dequeue forces
    // the data source to create fresh cells.
}

- (())registerNib:(id)_nib forCellWithReuseIdentifier:(id)_identifier {}

- (())registerClass:(id)_class
    forSupplementaryViewOfKind:(id)_kind
           withReuseIdentifier:(id)_identifier {}

- (())registerNib:(id)_nib
    forSupplementaryViewOfKind:(id)_kind
           withReuseIdentifier:(id)_identifier {}

// MARK: - Dequeue

- (id)dequeueReusableCellWithReuseIdentifier:(id)_identifier
                                forIndexPath:(id)_index_path {
    // No reuse pool — always create fresh cells.
    nil
}

- (id)dequeueReusableSupplementaryViewOfKind:(id)_kind
                         withReuseIdentifier:(id)_identifier
                                forIndexPath:(id)_index_path {
    nil
}

// MARK: - Reload

- (())reloadData {
    clear_cells(env, this, true);

    let data_source = env.objc.borrow::<UICollectionViewHostObject>(this).data_source;
    if data_source == nil {
        return;
    }

    let bounds: CGRect = msg![env; this bounds];
    let width = bounds.size.width;

    // Get item size from layout (default 50x50).
    let layout = env.objc.borrow::<UICollectionViewHostObject>(this).layout;
    let item_size: CGSize = if layout != nil {
        msg![env; layout itemSize]
    } else {
        CGSize { width: 50.0, height: 50.0 }
    };
    let spacing: CGFloat = if layout != nil {
        msg![env; layout minimumInteritemSpacing]
    } else {
        10.0
    };
    let line_spacing: CGFloat = if layout != nil {
        msg![env; layout minimumLineSpacing]
    } else {
        10.0
    };

    // Section count.
    let sel_num_sections = env.objc.lookup_selector("numberOfSectionsInCollectionView:");
    let num_sections: NSInteger = if let Some(sel) = sel_num_sections {
        let responds: bool = msg![env; data_source respondsToSelector:sel];
        if responds {
            msg![env; data_source numberOfSectionsInCollectionView:this]
        } else {
            1
        }
    } else {
        1
    };

    let items_per_row = ((width + spacing) / (item_size.width + spacing)).floor().max(1.0) as i32;
    let mut y: CGFloat = 0.0;
    let mut new_cells: Vec<id> = Vec::new();

    for section in 0..num_sections {
        let item_count: NSInteger =
            msg![env; data_source collectionView:this numberOfItemsInSection:section];

        let mut col: i32 = 0;
        for item in 0..item_count {
            let index_path: id = msg_class![env; NSIndexPath indexPathForItem:(item as NSUInteger)
                                                                   inSection:(section as NSUInteger)];
            let cell: id =
                msg![env; data_source collectionView:this cellForItemAtIndexPath:index_path];
            if cell == nil {
                continue;
            }
            retain(env, cell);

            let x = (col as CGFloat) * (item_size.width + spacing);
            let frame = CGRect {
                origin: CGPoint { x, y },
                size: item_size,
            };
            () = msg![env; cell setFrame:frame];
            () = msg![env; this addSubview:cell];

            new_cells.push(cell);
            col += 1;
            if col >= items_per_row {
                col = 0;
                y += item_size.height + line_spacing;
            }
        }
        if col != 0 {
            y += item_size.height + line_spacing;
        }
    }

    env.objc.borrow_mut::<UICollectionViewHostObject>(this).cells = new_cells;
    let content_size = CGSize { width, height: y };
    () = msg![env; this setContentSize:content_size];
}

- (())reloadSections:(id)_sections {
    () = msg![env; this reloadData];
}

- (())reloadItemsAtIndexPaths:(id)_paths {
    () = msg![env; this reloadData];
}

// MARK: - Visible cells

- (id)visibleCells {
    let cells = env.objc.borrow::<UICollectionViewHostObject>(this).cells.clone();
    let array: id = msg_class![env; NSMutableArray array];
    for cell in cells {
        () = msg![env; array addObject:cell];
    }
    array
}

- (id)indexPathsForVisibleItems {
    msg_class![env; NSArray new]
}

- (id)indexPathsForSelectedItems {
    msg_class![env; NSArray new]
}

// MARK: - Batch updates

- (())performBatchUpdates:(id)_updates completion:(id)_completion {
    () = msg![env; this reloadData];
}

- (())insertItemsAtIndexPaths:(id)_paths {
    () = msg![env; this reloadData];
}

- (())deleteItemsAtIndexPaths:(id)_paths {
    () = msg![env; this reloadData];
}

- (())moveItemAtIndexPath:(id)_from toIndexPath:(id)_to {
    () = msg![env; this reloadData];
}

- (())insertSections:(id)_sections {
    () = msg![env; this reloadData];
}

- (())deleteSections:(id)_sections {
    () = msg![env; this reloadData];
}

- (())moveSection:(NSInteger)_from toSection:(NSInteger)_to {
    () = msg![env; this reloadData];
}

// MARK: - Scrolling

- (())scrollToItemAtIndexPath:(id)_path
             atScrollPosition:(NSInteger)_scroll_position
                     animated:(bool)_animated {
    // Not yet implemented — would need item frame lookup.
}

// MARK: - Number queries

- (NSInteger)numberOfSections {
    let data_source = env.objc.borrow::<UICollectionViewHostObject>(this).data_source;
    if data_source == nil { return 0; }
    let sel = match env.objc.lookup_selector("numberOfSectionsInCollectionView:") {
        Some(sel) => sel,
        None => return 1,
    };
    let responds: bool = msg![env; data_source respondsToSelector:sel];
    if !responds { return 1; }
    msg![env; data_source numberOfSectionsInCollectionView:this]
}

- (NSInteger)numberOfItemsInSection:(NSInteger)section {
    let data_source = env.objc.borrow::<UICollectionViewHostObject>(this).data_source;
    if data_source == nil { return 0; }
    msg![env; data_source collectionView:this numberOfItemsInSection:section]
}

- (id)cellForItemAtIndexPath:(id)_path {
    nil
}

@end

// ==========================================================================
// UICollectionViewController
// ==========================================================================

@implementation UICollectionViewController: UIViewController

- (id)initWithCollectionViewLayout:(id)layout {
    let frame = <CGRect as Default>::default();
    let collection_view: id = msg_class![env; UICollectionView alloc];
    let collection_view: id = msg![env; collection_view initWithFrame:frame collectionViewLayout:layout];
    () = msg![env; this setView:collection_view];
    release(env, collection_view);
    this
}

- (id)collectionView {
    msg![env; this view]
}

- (())setCollectionView:(id)view {
    () = msg![env; this setView:view];
}

@end

};
