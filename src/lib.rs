#[cfg(test)]
mod borrows {
    use core::cell::UnsafeCell;
    use core::ptr;

    // Miri extern functions for inspecting the borrow state.
    #[cfg(miri)]
    extern "Rust" {
        fn miri_get_alloc_id(ptr: *const u8) -> u64;
        fn miri_print_borrow_state(alloc_id: u64, show_unnamed: bool);
    }

    /// Retrieve the unique identifier for the allocation pointed to by `ptr`.
    #[cfg(miri)]
    fn get_alloc_id(ptr: *const u8) -> u64 {
        unsafe { miri_get_alloc_id(ptr) }
    }

    /// Print (from the Miri interpreter) the contents of all borrows in an allocation.
    #[cfg(miri)]
    fn dbg_borrows(alloc_id: u64) {
        println!();
        unsafe { miri_print_borrow_state(alloc_id, true) }
    }

    #[test]
    #[cfg(miri)]
    fn test_dbg() {
        let val = 1_u8;
        let alloc = get_alloc_id(&val as *const u8);
        dbg_borrows(alloc);
    }

    /// This is UB under Tree Borrows but not UB under Stacked Borrows.
    #[test]
    fn test_2phase() {
        struct Wrap(u8);
        impl Wrap {
            fn action(&mut self, _arg: u8) {}
        }

        let mut x = Wrap(1);
        let y = &mut x.0 as *mut u8;
        x.action({
            unsafe { *y = 2 };
            x.0
        });
    }

    /// This is UB under both Stacked Borrows and Tree Borrows.
    #[test]
    fn test_protected() {
        fn protect<T, F: FnMut()>(_x: &mut T, mut f: F) {
            f();
        }
        let mut val = 1_u8;
        let x = &mut val;
        let y = x as *mut u8;
        let f = || unsafe { *y = 2 };
        protect(x, f);
    }

    /// This is UB under Stacked Borrows but ok under Tree Borrows.
    /// See [#257](https://github.com/rust-lang/unsafe-code-guidelines/issues/257)
    #[test]
    fn test_const_write() {
        let mut val = 1_u8;
        let x = &mut val as *const u8 as *mut u8;
        unsafe { *x = 2 }; // UB
    }

    /// This is ok under both Stacked Borrows and Tree Borrows.
    /// Under SB, the initial cast from &mut to *const / *mut determines
    /// whether or not it is UB to write through the pointer.
    #[test]
    fn test_ok_const_write() {
        let mut val = 1_u8;
        let x = &mut val as *mut u8 as *const u8 as *mut u8;
        unsafe { *x = 2 };
    }

    /// This is UB under Stacked Borrows but ok under Tree Borrows.
    #[test]
    fn test_reserved() {
        let mut val = 1_u8;
        let x = &mut val;
        let y = unsafe { &mut *(x as *mut u8) };
        // Under SB, this performs a dummy read granted by x which disables y.
        let s = &*x;
        assert_eq!(*s, 1);
        *y = 2;
        assert_eq!(val, 2);
    }

    /// This is UB under Stacked Borrows but ok under Tree Borrows.
    #[test]
    fn test_copy_nonoverlapping() {
        let mut val = [1_u8, 2];
        let src = val.as_ptr();
        let dst = unsafe { val.as_mut_ptr().add(1) };
        unsafe { ptr::copy_nonoverlapping(src, dst, 1) };
    }

    /// This is ok under both Stacked Borrows and Tree Borrows.
    #[test]
    fn test_ok_copy_nonoverlapping() {
        let mut val = [1_u8, 2];
        let dst = unsafe { val.as_mut_ptr().add(1) };
        // Under SB, this disables the Unique that dst is derive from, but the
        // SharedReadWrite dst is still valid.
        let src = val.as_ptr();
        unsafe { ptr::copy_nonoverlapping(src, dst, 1) };
    }

    /// This is UB under Stacked Borrows but ok under Tree Borrows.
    /// See [#134](https://github.com/rust-lang/unsafe-code-guidelines/issues/134).
    #[test]
    fn test_raw_ptr_restricted() {
        let val = [1_u8, 2];
        let ptr = &val[0] as *const u8;
        let _v = unsafe { *ptr.add(1) };
    }

    /// This is UB under Stacked Borrows but ok under Tree Borrows.
    /// Under SB, the parent of `_y` is `val`, which pops x off the stack.
    #[test]
    fn test_steal_borrow() {
        let mut val = 1_u8;
        let x: *mut u8 = &mut val;
        let _y: *mut u8 = &mut val; // not ok
        let _ = unsafe { *x }; // UB
    }

    /// This is ok under both Stacked Borrows and Tree Borrows.
    /// Under SB, the parent of `_y` is now `x`, so `x` stays on the stack.
    #[test]
    fn test_ok_steal_borrow() {
        let mut val = 1_u8;
        let x: *mut u8 = &mut val;
        let _y: *mut u8 = unsafe { &mut *x }; // ok
        let _ = unsafe { *x };
    }

    /// This is UB under Stacked Borrows but ok under Tree Borrows.
    /// Under SB, reading x disables y, and Disabled does not grant read access.
    #[test]
    fn test_disable_unique() {
        let mut val = 1_u8;
        let x: *mut u8 = &mut val;
        let y = unsafe { &mut *x };
        // This disables the Unique y.
        let _ = unsafe { ptr::read(x) };
        let _z = *y; // UB
    }

    /// This is ok under both Stacked Borrows and Tree Borrows.
    /// Under SB, y is now a SharedReadWrite derived from the disabled Unique.
    #[test]
    fn test_ok_disable_unique() {
        let mut val = 1_u8;
        let x: *mut u8 = &mut val;
        let y: *mut u8 = unsafe { &mut *x };
        // This disables the Unique that y is derived from, but not y itself.
        let _ = unsafe { ptr::read(x) };
        let _z = unsafe { *y };
    }

    /// This is UB under both Stacked Borrows and Tree Borrows.
    /// See [#450](https://github.com/rust-lang/unsafe-code-guidelines/issues/450)
    #[test]
    fn test_sibling_ptr() {
        let mut arr = [0_u8; 8];
        let ptr1 = arr.as_mut_ptr();
        let ptr2 = arr.as_mut_ptr();
        unsafe { ptr1.write(1) };
        unsafe { ptr2.write(1) };
    }

    #[test]
    fn test_cell() {
        let x = 0_usize;
        let cell = &x as *const usize as *const UnsafeCell<usize>;
        let _ = unsafe { &*cell }; // UB
    }

    #[test]
    fn test_ok_interleave_reads() {
        let mut val = 1_u8;
        let u: *mut u8 = &mut val;
        let s: *const u8 = unsafe { &*u };
        let _ = unsafe { *s };
        let _ = unsafe { *u };
        let _ = unsafe { *s };
    }
}

#[cfg(test)]
mod ptr {
    use core::ptr;

    #[test]
    fn test_no_provenance() {
        let p = ptr::NonNull::<u8>::dangling();
        // memory access failed: 0x1[noalloc] is a dangling pointer (it has no provenance)
        let _ = unsafe { *p.as_ptr() }; // UB
    }

    #[test]
    fn test_oob() {
        let arr: &[u8; 4] = &[0, 1, 2, 3];
        let p = arr as *const u8;
        // 1 byte past the end is okay
        let q = unsafe { p.add(arr.len()) };
        // any further is UB: out-of-bounds pointer arithmetic
        let _ = unsafe { q.offset(1) }; // UB
    }

    #[test]
    fn test_use_oob() {
        let val: [u8; 0] = [];
        let unaligned = val.as_ptr();
        let _: u8 = unsafe { *unaligned }; // UB
    }

    /// Run with `-Zmiri-symbolic-alignment-check` to reliably detect UB.
    #[test]
    fn test_unaligned() {
        // u32 has alignment 4 (on most platforms).
        #[repr(C, packed)]
        struct Packed(pub u8, pub u32);

        let packed = Packed(1, 2);
        let unaligned = &raw const packed.1;
        let _ = unsafe { *unaligned }; // UB
    }

    /// Run with `-Zmiri-symbolic-alignment-check` to reliably detect UB.
    #[test]
    fn test_unaligned_ref() {
        #[repr(align(2))]
        struct Foo<T>(T);

        let val = [1_u8, 2];
        let unaligned = val.as_ptr() as *const Foo<u8>;
        let _ = unsafe { &*unaligned }; // UB
    }

    /// Run with `-Zmiri-symbolic-alignment-check` to reliably detect UB.
    ///
    /// See https://www.ralfj.de/blog/2024/08/14/places.html
    #[test]
    fn test_place_expression() {
        #[repr(C, packed)]
        struct Struct {
            field: u32
        }
        let x = Struct { field: 42 };
        let ptr = &raw const x.field;
        // This is ok (safe, even)
        let _ptr_copy = &raw const *ptr;
        // This is UB
        let _val = unsafe { *ptr };
    }

    #[test]
    fn test_underscore_place() {
        let ptr = ptr::null::<i32>();
        // This is ok
        unsafe {
            let _ = *ptr;
        }
        // This is UB
        let _ = unsafe { *ptr };
    }

    #[test]
    fn test_deref_fn_ptr() {
        fn f() {}
        let p = f as *const u8;
        let _v = unsafe { *p }; // UB
    }

    #[test]
    fn test_double_drop() {
        let x = Box::new(1);
        let _y = unsafe { ptr::read(&x) };
        panic!("bad place to panic"); // UB
    }
}


#[cfg(test)]
mod concurrency {
    use std::thread;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering::{Acquire, Release, Relaxed};
    use std::sync::Arc;

    #[test]
    fn test_data_race() {
        static mut GLOBAL: usize = 0;

        let j1 = thread::spawn(|| unsafe { GLOBAL = 1 });
        let j2 = thread::spawn(|| unsafe { GLOBAL = 2 });
        j1.join().unwrap();
        j2.join().unwrap();
    }

    #[test]
    fn test_release_sequence() {
        #[derive(Clone, Copy)]
        struct BadSend<T>(pub T);
        unsafe impl<T> Send for BadSend<T> {}
        unsafe impl<T> Sync for BadSend<T> {}

        let mut val = 0usize;
        let x = BadSend(&mut val as *mut usize);
        let flag = Arc::new(AtomicBool::new(false));

        let j1 = {
            let flag = flag.clone();
            thread::spawn(move || {
                let x = x;
                unsafe { *x.0 = 1 };
                flag.store(true, Release);
                // The store below breaks the release sequence headed by the
                // store above.
                flag.store(true, Relaxed);
            })
        };
        let j2 = thread::spawn(move || {
            if flag.load(Acquire) {
                let x = x;
                let v = unsafe { *x.0 };
                assert_eq!(v, 1);
            }
        });
        j1.join().unwrap();
        j2.join().unwrap();
    }
}

#[cfg(test)]
mod validity {
    use core::mem;

    #[test]
    fn test_bad_bool() {
        let x = 2_u8;
        #[allow(clippy::transmute_int_to_bool)]
        let _: bool = unsafe { mem::transmute(x) };
    }
}
