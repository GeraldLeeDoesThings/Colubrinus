use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::cmp::min;
use core::ptr::{null_mut};

const STACK_SIZE_CONST: usize = 100000;
static STACK_SIZE: usize = STACK_SIZE_CONST;
static mut STACK: [u8; STACK_SIZE_CONST] = [0; STACK_SIZE_CONST];
const HEAP_SIZE_CONST: usize = 2000000;
static HEAP_SIZE: usize = HEAP_SIZE_CONST;
static mut HEAP: UnsafeCell<[u8; HEAP_SIZE_CONST]> = UnsafeCell::new([0; HEAP_SIZE_CONST]);


// This must be synchronized to support multithreading
struct Heap {}

#[global_allocator]
static ALLOCATOR: Heap = Heap {};

impl Heap {

    unsafe fn write_usize32(&self, at: *mut u8, val: usize) {
        at.write(((val & 0xFF000000) >> 24) as u8);
        at.add(1).write(((val & 0x00FF0000) >> 16) as u8);
        at.add(2).write(((val & 0x0000FF00) >> 8) as u8);
        at.add(3).write((val & 0x000000FF) as u8);
    }
    
    unsafe fn read_usize32(&self, at: *const u8) -> usize {
        return ((at.read() as usize) << 24) +
            ((at.add(1).read() as usize) << 16) +
            ((at.add(2).read() as usize) << 8) +
            (at.add(3).read() as usize);
    }

    unsafe fn format_cell(
        &self,
        at: *mut u8,
        size: usize,
        allocated: bool,
        prev_offset: usize,
        next_offset: usize
    ) {
        // Note that at is a pointer to the first byte of the CELL, not used memory
        self.write_usize32(at, size);
        at.add(4).write(allocated as u8);
        self.write_usize32(at.add(5), prev_offset);
        self.write_usize32(at.add(9), next_offset);
    }

    #[inline(always)]
    unsafe fn read_cell_size(&self, at: *mut u8) -> usize {
        return self.read_usize32(at.sub(4));
    }

    #[inline(always)]
    unsafe fn read_cell_prev_offset(&self, at: *mut u8) -> usize {
        return self.read_usize32(at.add(1));
    }

    #[inline(always)]
    unsafe fn read_cell_next_offset(&self, at: *mut u8) -> usize {
        return self.read_usize32(at.add(5));
    }

    #[inline(always)]
    unsafe fn write_cell_size(&self, at: *mut u8, val: usize) {
        self.write_usize32(at.sub(4), val);
    }

    #[inline(always)]
    unsafe fn write_cell_prev_offset(&self, at: *mut u8, val: usize) {
        self.write_usize32(at.add(1), val);
    }

    #[inline(always)]
    unsafe fn write_cell_next_offset(&self, at: *mut u8, val: usize) {
        self.write_usize32(at.add(5), val);
    }

    unsafe fn find_adjacent_free_cell(&self, at: *mut u8) -> *mut u8 {
        let mut heap_ptr: *mut u8 = HEAP.get() as *mut u8;
        let mut next_offset: usize = self.read_usize32(heap_ptr);
        if next_offset == 0 {
            return null_mut();
        }
        heap_ptr = heap_ptr.add(next_offset);
        next_offset = self.read_cell_next_offset(heap_ptr);
        while next_offset > 0 && heap_ptr.add(next_offset - 4) < at {
            heap_ptr = heap_ptr.add(next_offset);
            next_offset = self.read_cell_next_offset(heap_ptr);
        }
        return heap_ptr;
    }

    unsafe fn fix_offset_pair(&self, small: *mut u8, large: *mut u8) {
        let small_next: usize = large.offset_from(small) as usize;
        self.write_cell_next_offset(small, small_next);
        self.write_cell_prev_offset(large, small_next);
    }

    unsafe fn fix_offset_triple(&self, small: *mut u8, mid: *mut u8, large: *mut u8) {
        let small_next: usize = mid.offset_from(small) as usize;
        let mid_next: usize = large.offset_from(mid) as usize;
        self.write_cell_next_offset(small, small_next);
        self.write_cell_prev_offset(mid, small_next);
        self.write_cell_next_offset(mid, mid_next);
        self.write_cell_prev_offset(large, mid_next);
    }

    unsafe fn free_cell(&self, at: *mut u8) {
        // Note that at is a pointer to the first byte of USED memory, not the start of the cell
        let mut alloc_bit_offset: usize = self.get_alloc_bit_offset(at);
        let current_alloc_addr: *mut u8 = at.sub(alloc_bit_offset);
        at.sub(alloc_bit_offset).write(0);

        // Find the closest free cell with a lower address
        let prev_cell: *mut u8 = self.find_adjacent_free_cell(at);

        if !prev_cell.is_null() {
            // A cell was found, store the offset to the next cell
            let next_offset: usize = self.read_cell_next_offset(prev_cell);

            if next_offset > 0 {
                // If the previous cell was pointing to another cell, that cell must be the
                // closest free cell with a larger address, so update its prev offset
                let next_cell = prev_cell.add(next_offset);
                self.fix_offset_triple(prev_cell, current_alloc_addr, next_cell);
            }
            else {
                self.fix_offset_pair(prev_cell, current_alloc_addr);
                // There is no larger cell, mark this by setting offset to 0
                self.write_cell_next_offset(current_alloc_addr, 0);
            }
        }
        else {
            // There is no free cell with a smaller address, so point the initial offset here
            let heap_ptr: *mut u8 = HEAP.get() as *mut u8;
            let next_offset: usize = self.read_usize32(heap_ptr);

            // If the initial offset pointed somewhere, update its previous offset
            if next_offset > 0 {
                let next_cell = heap_ptr.add(next_offset);
                self.fix_offset_pair(current_alloc_addr, next_cell);
            }
            self.write_usize32(heap_ptr, current_alloc_addr.offset_from(heap_ptr) as usize);
        }
        self.try_merge(current_alloc_addr);
    }

    unsafe fn get_first_cell_byte(&self, cell: *mut u8) -> *mut u8 {
        return cell.sub(4);
    }

    unsafe fn get_last_cell_byte(&self, cell: *mut u8) -> *mut u8 {
        return cell.add(self.read_cell_size(cell));
    }

    unsafe fn are_adjacent(&self, cell_a: *mut u8, cell_b: *mut u8) -> bool {
        return min(
            self.get_last_cell_byte(cell_a).offset_from(
                self.get_first_cell_byte(cell_b)
            ),
            self.get_first_cell_byte(cell_a).offset_from(
                self.get_last_cell_byte(cell_b)
            )
        ) < 13;
    }

    unsafe fn merge(&self, small: *mut u8, large: *mut u8) {
        self.write_cell_size(
            small,
            self.get_last_cell_byte(large).offset_from(small) as usize - 1
        );
        let old_next_offset: usize = self.read_cell_next_offset(large);
        if old_next_offset > 0 {
            self.fix_offset_pair(small, large.add(old_next_offset));
        }
    }

    unsafe fn try_merge(&self, at: *mut u8) -> bool {
        let prev_offset: usize = self.read_cell_prev_offset(at);
        let next_offset: usize = self.read_cell_next_offset(at);
        if prev_offset > 0 && self.are_adjacent(at.sub(prev_offset), at) {
            self.merge(at.sub(prev_offset), at);
            self.try_merge(at.sub(prev_offset));
            return true;
        }
        if next_offset > 0 && self.are_adjacent(at.add(next_offset), at) {
            self.merge(at, at.add(next_offset));
            self.try_merge(at);
            return true;
        }
        return false;
    }

    unsafe fn setup(&self) {
        // Allocate all memory as a single cell
        let heap_ptr: *mut u8 = HEAP.get() as *mut u8;
        self.write_usize32(heap_ptr, 5); // Initial offset to first free cell
        self.format_cell(
            heap_ptr.add(4),
            HEAP_SIZE - 9, // 1 byte from alloc byte, 4 each from initial offset & size itself
            false,
            0,
            0
        ); // Write a cell containing all remaining memory
    }

    unsafe fn claim(&self, at: *mut u8) {
        let prev_offset: usize = self.read_cell_prev_offset(at);
        let next_offset: usize = self.read_cell_next_offset(at);
        at.write(1);
        if prev_offset > 0 {
            if next_offset > 0 {
                self.fix_offset_pair(at.sub(prev_offset), at.add(next_offset));
            }
            else {
                self.write_cell_next_offset(at.sub(prev_offset), 0);
            }
        }
        else if next_offset > 0 {
            let heap_ptr: *mut u8 = HEAP.get() as *mut u8;
            self.write_usize32(
                heap_ptr, at.add(next_offset).offset_from(heap_ptr) as usize
            );
            self.write_cell_prev_offset(at.add(next_offset), 0);
        }
        else {
            self.write_usize32(HEAP.get() as *mut u8, 0);
        }
    }

    unsafe fn shrink(&self, cellptr: *mut u8, tosize: usize) -> bool {
        let cursize: usize = self.read_cell_size(cellptr);
        return if cursize < tosize {
            false
        } else if cursize == tosize {
            true
        } else if cursize - tosize >= 14 {
            self.write_cell_size(cellptr, tosize);
            self.format_cell(
                cellptr.add(tosize + 1),
                cursize - tosize - 13,
                false, 0,
                0
            );
            if self.has_next_cell(cellptr) {
                self.fix_offset_triple(
                    cellptr,
                    cellptr.add(tosize + 5),
                    cellptr.add(self.read_cell_next_offset(cellptr))
                );
            } else {
                self.fix_offset_pair(cellptr, cellptr.add(tosize + 5));
            }
            true
        } else {
            // Refuse to shrink, cannot create another cell with that memory
            true
        }
    }

    unsafe fn get_alloc_bit_offset(&self, from: *mut u8) -> usize {
        // Note that at is a pointer to the first byte of USED memory, not the start of the cell
        let mut alloc_bit_offset: usize = 1;
        while from.sub(alloc_bit_offset).read() != 1 {
            alloc_bit_offset += 1;
        }
        return alloc_bit_offset;
    }

    #[inline(always)]
    unsafe fn has_next_cell(&self, cell: *mut u8) -> bool {
        return self.read_cell_next_offset(cell) > 0;
    }

    #[inline(always)]
    unsafe fn has_prev_cell(&self, cell: *mut u8) -> bool {
        return self.read_cell_prev_offset(cell) > 0;
    }

}

unsafe impl GlobalAlloc for Heap {

    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut heap_ptr: *mut u8 = HEAP.get() as *mut u8;
        let mut next_offset: usize = self.read_usize32(heap_ptr);
        heap_ptr = heap_ptr.add(next_offset);
        let mut padding: usize = (heap_ptr as usize) % layout.align();
        while self.read_cell_size(heap_ptr) < layout.size() + padding {
            next_offset = self.read_cell_next_offset(heap_ptr);
            if next_offset == 0 {
                // No cells are large enough
                return null_mut();
            }
            heap_ptr = heap_ptr.add(next_offset);
            padding = (heap_ptr as usize) % layout.align();
        }

        if !self.shrink(heap_ptr, layout.size() + padding) {
            // Something is very wrong with the shrink, should never happen
            return null_mut();
        }
        self.claim(heap_ptr);

        if padding > 0 {
            for i in 1..padding + 1 {
                heap_ptr.add(i).write(0);
            }
        }
        return heap_ptr.add(padding + 1);
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        self.free_cell(ptr);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let result: *mut u8 = self.alloc(layout);
        for i in 1..layout.size() + 1 {
            result.add(i).write(0);
        }
        return result;
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Attempt to use part of the next cell to realloc without copy
        let alloc_bit_offset: usize = self.get_alloc_bit_offset(ptr);
        let cursize: usize = self.read_cell_size(ptr.sub(alloc_bit_offset));
        if cursize >= new_size {
            return ptr;
        }
        let maybenext: *mut u8 = ptr.sub(alloc_bit_offset).add(cursize + 5);
        // If the next cell isn't also in use
        if maybenext.read() == 0 {
            // Shrink it to size to minimize space wastage
            if self.shrink(maybenext, new_size - cursize) {
                self.claim(maybenext);
                self.write_cell_size(ptr, cursize + 5 + self.read_cell_size(maybenext));
                return ptr;
            }
            // Not enough space, must attempt with copy
        }

        let nlayout = Layout::from_size_align(new_size, layout.align());
        let n: *mut u8;
        match nlayout {
            Ok(lay) => n = self.alloc(lay),
            Err(_) => return null_mut()
        };

        // Copy over old data
        for i in 1..layout.size() + 1 {
            n.add(i).write(ptr.add(i).read())
        }

        self.dealloc(ptr, layout);
        return n;
    }

}
