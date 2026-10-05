/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSSet` class cluster, including `NSMutableSet` and `NSCountedSet`.

use super::ns_array;
use super::ns_dictionary::DictionaryHostObject;
use super::ns_enumerator::{fast_enumeration_helper, NSFastEnumerationState};
use super::ns_keyed_archiver::{encode_object, get_value_to_encode_for_current_key};
use super::ns_keyed_unarchiver;
use super::NSUInteger;
use crate::abi::DotDotDot;
use crate::environment::Environment;
use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, nil, objc_classes, release, retain, ClassExports,
    HostObject, NSZonePtr, SEL,
};

/// Belongs to _touchHLE_NSSet
#[derive(Debug, Default)]
struct SetHostObject {
    dict: DictionaryHostObject,
}
impl HostObject for SetHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSSet is an abstract class. A subclass must provide:
// - (NSUInteger)count;
// - (id)member:(id)object;
// - (NSEnumerator*)objectEnumerator;
// We can pick whichever subclass we want for the various alloc methods.
// For the time being, that will always be _touchHLE_NSSet.
@implementation NSSet: NSObject

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSSet might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSSet", &mut env.mem));
    msg_class![env; _touchHLE_NSSet allocWithZone:zone]
}

+ (id)set {
    let set: id = msg![env; this new];
    autorelease(env, set)
}

+ (id)setWithArray:(id)array { // NSArray *
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithArray:array];
    autorelease(env, new)
}

+ (id)setWithObject:(id)object {
    assert!(object != nil);
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithObject:object];
    autorelease(env, new)
}

+ (id)setWithObjects:(id)first_obj, ...args {
    assert!(this == env.objc.get_known_class("NSSet", &mut env.mem));
    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<SetHostObject>(new).dict = set_from_objects(env, first_obj, args);
    autorelease(env, new)
}

+ (id)setWithSet:(id)set { // NSSet*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithSet:set];
    autorelease(env, new)
}

- (id)initWithSet:(id)set { // NSSet*
    let objects: id = msg![env; set allObjects];
    msg![env; this initWithArray:objects]
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

// NSMutableCopying implementation
- (id)mutableCopyWithZone:(NSZonePtr)_zone {
    let new: id = msg_class![env; NSMutableSet alloc];
    msg![env; new initWithSet:this]
}

// NSCoding implementation
// Sets are archived like arrays, with an "NS.objects" array.
// TODO: support other types of coders, not only NSKeyedArchiver
- (id)initWithCoder:(id)coder {
    // The objects are retained by the Vec, and then by the array.
    let objects = ns_keyed_unarchiver::decode_current_array(env, coder);
    let array = ns_array::from_vec(env, objects);
    let new: id = msg![env; this initWithArray:array];
    release(env, array);
    new
}
- (())encodeWithCoder:(id)coder {
    let objects: id = msg![env; this allObjects];
    let count: NSUInteger = msg![env; objects count];
    let mut encoded = Vec::with_capacity(count as usize);
    for i in 0..count {
        let object: id = msg![env; objects objectAtIndex:i];
        encoded.push(plist::Value::Uid(encode_object(env, coder, object)));
    }
    let scope = get_value_to_encode_for_current_key(env, coder);
    scope.insert("NS.objects".to_string(), plist::Value::Array(encoded));
}

- (bool)containsObject:(id)object {
    let member: id = msg![env; this member:object];
    member != nil
}

- (id)member:(id)object {
    let enumerator: id = msg![env; this objectEnumerator];
    loop {
        let next: id = msg![env; enumerator nextObject];
        if next == nil {
            return nil;
        }
        if msg![env; next isEqual:object] {
            return next;
        }
    }
}

- (bool)isSubsetOfSet:(id)other { // NSSet*
    let enumerator: id = msg![env; this objectEnumerator];
    loop {
        let next: id = msg![env; enumerator nextObject];
        if next == nil {
            return true;
        }
        if !msg![env; other containsObject:next] {
            return false;
        }
    }
}

- (bool)intersectsSet:(id)other { // NSSet*
    let enumerator: id = msg![env; this objectEnumerator];
    loop {
        let next: id = msg![env; enumerator nextObject];
        if next == nil {
            return false;
        }
        if msg![env; other containsObject:next] {
            return true;
        }
    }
}

- (bool)isEqualToSet:(id)other { // NSSet*
    let count: NSUInteger = msg![env; this count];
    let other_count: NSUInteger = msg![env; other count];
    count == other_count && msg![env; this isSubsetOfSet:other]
}

- (id)setByAddingObject:(id)object {
    let new: id = msg![env; this mutableCopy];
    () = msg![env; new addObject:object];
    immutable_copy_and_release(env, new)
}

- (id)setByAddingObjectsFromSet:(id)other { // NSSet*
    let new: id = msg![env; this mutableCopy];
    () = msg![env; new unionSet:other];
    immutable_copy_and_release(env, new)
}

- (id)setByAddingObjectsFromArray:(id)array { // NSArray*
    let new: id = msg![env; this mutableCopy];
    () = msg![env; new addObjectsFromArray:array];
    immutable_copy_and_release(env, new)
}

- (())makeObjectsPerformSelector:(SEL)selector {
    let objects: id = msg![env; this allObjects];
    let count: NSUInteger = msg![env; objects count];
    for i in 0..count {
        let object: id = msg![env; objects objectAtIndex:i];
        () = msg_send(env, (object, selector));
    }
}

- (())makeObjectsPerformSelector:(SEL)selector withObject:(id)argument {
    let objects: id = msg![env; this allObjects];
    let count: NSUInteger = msg![env; objects count];
    for i in 0..count {
        let object: id = msg![env; objects objectAtIndex:i];
        () = msg_send(env, (object, selector, argument));
    }
}

@end

// NSMutableSet is an abstract class. A subclass must provide everything
// NSSet provides, plus:
// - (void)addObject:(id)object;
// - (void)removeObject:(id)object;
// Note that it inherits from NSSet, so we must ensure we override any default
// methods that would be inappropriate for mutability.
@implementation NSMutableSet: NSSet

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSSet might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSMutableSet", &mut env.mem));
    msg_class![env; _touchHLE_NSMutableSet allocWithZone:zone]
}

+ (id)setWithObjects:(id)first_obj, ...args {
    assert!(this == env.objc.get_known_class("NSMutableSet", &mut env.mem));
    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<SetHostObject>(new).dict = set_from_objects(env, first_obj, args);
    autorelease(env, new)
}

+ (id)setWithCapacity:(NSUInteger)capacity {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithCapacity:capacity];
    autorelease(env, new)
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    let new: id = msg_class![env; NSSet alloc];
    msg![env; new initWithSet:this]
}

- (())addObjectsFromArray:(id)array { // NSArray*
    let count: NSUInteger = msg![env; array count];
    for i in 0..count {
        let object: id = msg![env; array objectAtIndex:i];
        () = msg![env; this addObject:object];
    }
}

- (())minusSet:(id)other { // NSSet*
    let objects: id = msg![env; other allObjects];
    let count: NSUInteger = msg![env; objects count];
    for i in 0..count {
        let object: id = msg![env; objects objectAtIndex:i];
        () = msg![env; this removeObject:object];
    }
}

- (())intersectSet:(id)other { // NSSet*
    let objects: id = msg![env; this allObjects];
    let count: NSUInteger = msg![env; objects count];
    for i in 0..count {
        let object: id = msg![env; objects objectAtIndex:i];
        if !msg![env; other containsObject:object] {
            () = msg![env; this removeObject:object];
        }
    }
}

- (())setSet:(id)other { // NSSet*
    () = msg![env; this removeAllObjects];
    () = msg![env; this unionSet:other];
}

@end

// Our private subclass that is the single implementation of NSSet for the
// time being.
@implementation _touchHLE_NSSet: NSSet

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SetHostObject {
        dict: Default::default(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithObject:(id)object {
    let null: id = msg_class![env; NSNull null];

    let mut dict = <DictionaryHostObject as Default>::default();
    dict.insert(env, object, null, /* copy_key: */ false);

    env.objc.borrow_mut::<SetHostObject>(this).dict = dict;

    this
}

- (id)initWithObjects:(id)first_obj, ...args {
    env.objc.borrow_mut::<SetHostObject>(this).dict = set_from_objects(env, first_obj, args);
    this
}

- (id)initWithArray:(id)array {
    env.objc.borrow_mut::<SetHostObject>(this).dict = set_from_array(env, array);
    this
}

- (())dealloc {
    std::mem::take(&mut env.objc.borrow_mut::<SetHostObject>(this).dict).release(env);
    env.objc.dealloc_object(this, &mut env.mem)
}

// TODO: more init methods, etc

// TODO: accessors
- (bool)containsObject:(id)object {
    let host_obj: SetHostObject = std::mem::take(env.objc.borrow_mut(this));
    let contained = host_obj.dict.lookup(env, object) != nil;
    *env.objc.borrow_mut(this) = host_obj;
    contained
}

- (NSUInteger)count {
    env.objc.borrow_mut::<SetHostObject>(this).dict.count
}

- (id)anyObject {
    let object_or_none = env.objc.borrow_mut::<SetHostObject>(this).dict.iter_keys().next();
    match object_or_none {
        Some(object) => object,
        None => nil
    }
}

- (id)allObjects {
    all_objects_common(env, this)
}

- (id)objectEnumerator { // NSEnumerator*
    let array: id = msg![env; this allObjects];
    msg![env; array objectEnumerator]
}

// NSFastEnumeration implementation
- (NSUInteger)countByEnumeratingWithState:(MutPtr<NSFastEnumerationState>)state
                                  objects:(MutPtr<id>)stackbuf
                                    count:(NSUInteger)len {
    // We assume that order in which objects are reported is consistent
    // between calls!
    let objects: id = msg![env; this allObjects];
    let count: NSUInteger = msg![env; objects count];
    fast_enumeration_helper(env, this, |env, idx| {
        if idx < count {
            msg![env; objects objectAtIndex:idx]
        } else {
            nil
        }
    }, state, stackbuf, len)
}

@end

// Our private subclass that is the single implementation of NSMutableSet for
// the time being.
@implementation _touchHLE_NSMutableSet: NSMutableSet

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SetHostObject {
        dict: Default::default(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithObject:(id)object {
    let null: id = msg_class![env; NSNull null];

    let mut dict = <DictionaryHostObject as Default>::default();
    dict.insert(env, object, null, /* copy_key: */ false);

    env.objc.borrow_mut::<SetHostObject>(this).dict = dict;

    this
}

- (id)initWithObjects:(id)first_obj, ...args {
    env.objc.borrow_mut::<SetHostObject>(this).dict = set_from_objects(env, first_obj, args);
    this
}

- (id)initWithArray:(id)array {
    env.objc.borrow_mut::<SetHostObject>(this).dict = set_from_array(env, array);
    this
}

- (id)initWithCapacity:(NSUInteger)_capacity {
    // TODO: capacity
    msg![env; this init]
}

- (())dealloc {
    std::mem::take(&mut env.objc.borrow_mut::<SetHostObject>(this).dict).release(env);
    env.objc.dealloc_object(this, &mut env.mem)
}

// TODO: init methods etc

- (bool)containsObject:(id)object {
    let host_obj: SetHostObject = std::mem::take(env.objc.borrow_mut(this));
    let contained = host_obj.dict.lookup(env, object) != nil;
    *env.objc.borrow_mut(this) = host_obj;
    contained
}

- (NSUInteger)count {
    env.objc.borrow_mut::<SetHostObject>(this).dict.count
}

- (id)anyObject {
    let object_or_none = env.objc.borrow_mut::<SetHostObject>(this).dict.iter_keys().next();
    match object_or_none {
        Some(object) => object,
        None => nil
    }
}

- (id)allObjects {
    all_objects_common(env, this)
}

- (id)objectEnumerator { // NSEnumerator*
    let array: id = msg![env; this allObjects];
    msg![env; array objectEnumerator]
}

// NSFastEnumeration implementation
- (NSUInteger)countByEnumeratingWithState:(MutPtr<NSFastEnumerationState>)state
                                  objects:(MutPtr<id>)stackbuf
                                    count:(NSUInteger)len {
    // TODO: check that set wasn't mutated!
    // We assume that order in which objects are reported is consistent
    // between calls!
    let objects: id = msg![env; this allObjects];
    let count: NSUInteger = msg![env; objects count];
    fast_enumeration_helper(env, this, |env, idx| {
        if idx < count {
            msg![env; objects objectAtIndex:idx]
        } else {
            nil
        }
    }, state, stackbuf, len)
}

// TODO: more mutation methods

- (())addObject:(id)object {
    let null: id = msg_class![env; NSNull null];
    let mut host_obj: SetHostObject = std::mem::take(env.objc.borrow_mut(this));
    host_obj.dict.insert(env, object, null, /* copy_key: */ false);
    *env.objc.borrow_mut(this) = host_obj;
}

- (())removeObject:(id)object {
    let mut host_obj: SetHostObject = std::mem::take(env.objc.borrow_mut(this));
    host_obj.dict.remove(env, object);
    *env.objc.borrow_mut(this) = host_obj;
}

- (())removeAllObjects {
    let mut old_host_obj = std::mem::replace(
        env.objc.borrow_mut(this),
        SetHostObject {
            dict: Default::default(),
        },
    );
    old_host_obj.dict.release(env);
}

- (())unionSet:(id)other { // NSSet *
    let enumerator: id = msg![env; other objectEnumerator];
    loop {
        let next: id = msg![env; enumerator nextObject];
        if next == nil {
            break;
        }
        () = msg![env; this addObject:next];
    }
}

@end

};

/// Shared implementation of `allObjects` for `_touchHLE_NSSet` and
/// `_touchHLE_NSMutableSet`.
fn all_objects_common(env: &mut Environment, this: id) -> id {
    let objects: Vec<id> = env
        .objc
        .borrow::<SetHostObject>(this)
        .dict
        .iter_keys()
        .collect();
    for &object in &objects {
        retain(env, object);
    }
    let array = ns_array::from_vec(env, objects);
    autorelease(env, array)
}

/// Make an autoreleased immutable copy of a mutable set and release the
/// original.
fn immutable_copy_and_release(env: &mut Environment, mutable_set: id) -> id {
    let new: id = msg![env; mutable_set copy];
    release(env, mutable_set);
    autorelease(env, new)
}

/// Helper method shared between `initWithObjects:` of `_touchHLE_NSSet` and
/// `_touchHLE_NSMutableSet`
fn set_from_objects(env: &mut Environment, first_obj: id, args: DotDotDot) -> DictionaryHostObject {
    let null: id = msg_class![env; NSNull null];

    let mut dict = <DictionaryHostObject as Default>::default();
    dict.insert(env, first_obj, null, /* copy_key: */ false);
    let mut varargs = args.start();
    loop {
        let next_arg: id = varargs.next(env);
        if next_arg == nil {
            break;
        }
        dict.insert(env, next_arg, null, /* copy_key: */ false);
    }
    dict
}

/// Helper method shared between `initWithArray:` of `_touchHLE_NSSet` and
/// `_touchHLE_NSMutableSet`
fn set_from_array(env: &mut Environment, array: id) -> DictionaryHostObject {
    let null: id = msg_class![env; NSNull null];

    let mut dict = <DictionaryHostObject as Default>::default();
    let count: NSUInteger = msg![env; array count];
    for i in 0..count {
        let next: id = msg![env; array objectAtIndex:i];
        dict.insert(env, next, null, /* copy_key: */ false);
    }
    dict
}
