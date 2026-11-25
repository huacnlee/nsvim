//! The C library of the web build. The tree-sitter parser and the grammar scanners are C, and a
//! browser gives C no library. The headers are in `web/libc/include`. This module supplies the
//! functions that the C code calls.
//!
//! The memory functions use the Rust allocator. Each block keeps its size in a header before
//! the pointer that C gets. The files and the clock do nothing: the parser writes to a file
//! only for a debug graph, and it reads the clock only for a time limit that the editor does
//! not set.
//!
//! On wasm32, a C call to a function with `...` gives the extra arguments as one pointer. So
//! `fprintf` and `snprintf` take that pointer as their last parameter.

use std::alloc::{Layout, alloc, alloc_zeroed, dealloc, realloc as grow};
use std::ffi::{c_char, c_int, c_void};
use std::ptr::null_mut;

/// The header before each block: its size. It also keeps the block aligned for any C type.
const HEAD: usize = 16;

fn layout(size: usize) -> Option<Layout> {
    Layout::from_size_align(size.checked_add(HEAD)?, HEAD).ok()
}

/// # Safety
/// `base` is a block from `alloc` with the layout of `size`.
unsafe fn start(base: *mut u8, size: usize) -> *mut c_void {
    if base.is_null() {
        return null_mut();
    }
    unsafe {
        (base as *mut usize).write(size);
        base.add(HEAD) as *mut c_void
    }
}

/// The block and size of a pointer that `malloc` gave.
///
/// # Safety
/// `p` came from `malloc`, `calloc` or `realloc` here.
unsafe fn block(p: *mut c_void) -> (*mut u8, usize) {
    unsafe {
        let base = (p as *mut u8).sub(HEAD);
        (base, (base as *const usize).read())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    let Some(l) = layout(size) else { return null_mut() };
    unsafe { start(alloc(l), size) }
}

#[unsafe(no_mangle)]
pub extern "C" fn calloc(n: usize, size: usize) -> *mut c_void {
    let Some(size) = n.checked_mul(size) else { return null_mut() };
    let Some(l) = layout(size) else { return null_mut() };
    unsafe { start(alloc_zeroed(l), size) }
}

/// # Safety
/// `p` is null or came from this allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn realloc(p: *mut c_void, size: usize) -> *mut c_void {
    if p.is_null() {
        return malloc(size);
    }
    let Some(new) = layout(size) else { return null_mut() };
    unsafe {
        let (base, old) = block(p);
        let old = layout(old).expect("a block that was made has a layout");
        start(grow(base, old, new.size()), size)
    }
}

/// # Safety
/// `p` is null or came from this allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    unsafe {
        let (base, size) = block(p);
        dealloc(base, layout(size).expect("a block that was made has a layout"));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn abort() -> ! {
    std::process::abort()
}

/// # Safety
/// `s` holds `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void {
    let bytes = unsafe { std::slice::from_raw_parts(s as *const u8, n) };
    match bytes.iter().position(|&b| b == c as u8) {
        Some(i) => unsafe { (s as *mut u8).add(i) as *mut c_void },
        None => null_mut(),
    }
}

/// # Safety
/// `a` and `b` end with a zero within `n` bytes, or hold `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int {
    for i in 0..n {
        let (x, y) = unsafe { (*a.add(i) as u8, *b.add(i) as u8) };
        if x != y || x == 0 {
            return x as c_int - y as c_int;
        }
    }
    0
}

/// # Safety
/// `a` and `b` end with a zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strcmp(a: *const c_char, b: *const c_char) -> c_int {
    unsafe { strncmp(a, b, usize::MAX) }
}

fn wide(c: u32, test: fn(char) -> bool) -> c_int {
    char::from_u32(c).is_some_and(test) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn iswalnum(c: u32) -> c_int {
    wide(c, char::is_alphanumeric)
}

#[unsafe(no_mangle)]
pub extern "C" fn iswalpha(c: u32) -> c_int {
    wide(c, char::is_alphabetic)
}

#[unsafe(no_mangle)]
pub extern "C" fn iswdigit(c: u32) -> c_int {
    wide(c, |c| c.is_ascii_digit())
}

#[unsafe(no_mangle)]
pub extern "C" fn iswlower(c: u32) -> c_int {
    wide(c, char::is_lowercase)
}

#[unsafe(no_mangle)]
pub extern "C" fn iswupper(c: u32) -> c_int {
    wide(c, char::is_uppercase)
}

#[unsafe(no_mangle)]
pub extern "C" fn iswspace(c: u32) -> c_int {
    wide(c, char::is_whitespace)
}

/// The case of `c` that `map` gives, when that is one character. Else `c` as it is.
fn case(c: u32, map: fn(char) -> Option<char>) -> u32 {
    char::from_u32(c).and_then(map).map_or(c, u32::from)
}

/// One character from a case map that gives exactly one.
fn single(mut it: impl Iterator<Item = char>) -> Option<char> {
    let c = it.next()?;
    it.next().is_none().then_some(c)
}

#[unsafe(no_mangle)]
pub extern "C" fn towlower(c: u32) -> u32 {
    case(c, |c| single(c.to_lowercase()))
}

#[unsafe(no_mangle)]
pub extern "C" fn towupper(c: u32) -> u32 {
    case(c, |c| single(c.to_uppercase()))
}

/// No clock: the parser has no time limit.
#[unsafe(no_mangle)]
pub extern "C" fn clock() -> i64 {
    0
}

/// No files: each call fails.
#[unsafe(no_mangle)]
pub extern "C" fn dup(_fd: c_int) -> c_int {
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn fdopen(_fd: c_int, _mode: *const c_char) -> *mut c_void {
    null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn fclose(_file: *mut c_void) -> c_int {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn fputc(c: c_int, _file: *mut c_void) -> c_int {
    c
}

#[unsafe(no_mangle)]
pub extern "C" fn fputs(_s: *const c_char, _file: *mut c_void) -> c_int {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn fwrite(_p: *const c_void, _size: usize, n: usize, _file: *mut c_void) -> usize {
    n
}

#[unsafe(no_mangle)]
pub extern "C" fn fprintf(_file: *mut c_void, _format: *const c_char, _args: *const c_void) -> c_int {
    0
}

/// Writes an empty string: the parser formats text only for debug output.
///
/// # Safety
/// `buf` holds `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vsnprintf(buf: *mut c_char, n: usize, _format: *const c_char, _args: *const c_void) -> c_int {
    if n > 0 && !buf.is_null() {
        unsafe { buf.write(0) };
    }
    0
}

/// # Safety
/// `buf` holds `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn snprintf(buf: *mut c_char, n: usize, format: *const c_char, args: *const c_void) -> c_int {
    unsafe { vsnprintf(buf, n, format, args) }
}
