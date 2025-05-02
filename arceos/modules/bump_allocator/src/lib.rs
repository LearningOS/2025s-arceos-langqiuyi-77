#![no_std]

use core::{alloc::Layout, marker::PhantomData, ptr::NonNull};

use allocator::{BaseAllocator, ByteAllocator, PageAllocator, AllocError};

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///

pub struct Bump<'pool> {
  pub start: usize,
  pub end: usize,
  pub bytes_pos: usize,
  pub page_pos: usize,
  _phantom: PhantomData<&'pool ()>, // TODO: 我不知道这个是个啥子, 就是仿照 Tlsf
  // 这是个 编译器提示工具，对运行时没有任何影响，主要是为了告诉 Rust 的借用检查器：
  // “这个结构体 Bump<'pool> 虽然看起来没用到 'pool 生命周期的数据，但其实跟 'pool 生命周期有关”。
  // PhantomData<&'pool ()> 看成是一个假的引用（ghost reference），它不占内存，也不影响运行时行为，只是告诉编译器：
  // “嘿，我这个 Bump 分配器其实是靠某个 'pool 生命周期的内存池活着的哦！”
  // 如果没有它，Rust 会认为 Bump 跟 'pool 没啥关系，可能导致你使用它时出现悬垂引用、不安全的生命周期扩展等编译错误。
  // 啊，这语法，，，，，，，，晕死我了，，，， 
}

impl <'pool> Bump<'pool> {
  pub const fn new() -> Self {
    Self {
      start: 0,
      end: 0,
      bytes_pos: 0,
      page_pos: 0,
      _phantom: {
          PhantomData
      },
    }
  }
}

pub struct EarlyAllocator<const PAGE_SIZE: usize> {
  // 有很多混乱的想法不清晰，首先肯定 EarlyAllocator 肯定是有一个数组来存储空间的方式, 所以我直接使用 Collections 中的 Vec ???
  // 当然不行，因为我们现在就是使用 EarlyAllocator 对上层的 Collection 做支持，是不能使用的
  // 那这里终究是要从 let pool = core::slice::from_raw_parts_mut(start as *mut u8, size);
  // 这里的 pool 就是 &mut [u8] 的效果，我们就是会有这个
  // FIXME: 那问题是对应的 add_memory 怎么实现？？？ 不实现 最小实现先，其他的管不了了
  inner: Bump<'static>,
  total_bytes: usize,
  used_bytes: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
  pub const fn new() -> Self {
    Self {
      inner: Bump::new(),
      total_bytes: 0,
      used_bytes: 0,
    }
  }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        unsafe {
            let pool = core::slice::from_raw_parts_mut(start as *mut u8, size);
            self.inner.start = pool.as_ptr() as usize;
            self.inner.end = self.inner.start + size;
            // 注意：Rust 的切片 &mut [u8] 不只是地址，它还包含长度信息，是一个“胖指针”。
            // 所以要先转换成为 .as_ptr() 指针，并且 as usize 方便直接做地址加减
            self.inner.bytes_pos = self.inner.start;
            self.inner.page_pos = self.inner.end;
        }
        self.total_bytes = size;
        self.used_bytes = 0;
    }

    fn add_memory(&mut self, _start: usize, _size: usize) -> allocator::AllocResult {
        todo!()
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(&mut self, layout: core::alloc::Layout) -> allocator::AllocResult<core::ptr::NonNull<u8>> {
        // 对应的 layout 有要求的 size, 还有对应需要的 align 的要求 
        // 理解 align 的要求：如果我要从 bump 分配器中分配这段 Layout，我从当前地址要往后跳多少字节才能对齐，然后加上 layout.size()，总共跳多少？
        let align = layout.align();
        let size = layout.size();

        // 1. 对齐当前指针到 layout.align() 
        // TODO: 经典的向上对齐的算法
        let aligned = (self.inner.bytes_pos + align - 1) & !(align - 1);

        // 2. 分配需要的大小
        let end = aligned.checked_add(size).ok_or(AllocError::NoMemory)?;
        // aligned.checked_add(size) 返回 Option<usize>
        // .ok_or(...) 把它转换成 Result<usize, AllocError>
        // ? 会自动返回 Err(...)，如果是 None
        // 如果是 Some(end)，则 end 就绑定为 usize，你可以在后面放心用
        if end > self.inner.page_pos {
            return Err(AllocError::NoMemory); // 空间不够
        }

        // 3. 你要返回的，是分配后对齐地址的指针，所以返回的是 aligned（作为地址）转换成 NonNull<u8>
        // 更新指针，返回 NonNull 指针
        self.used_bytes += (end - self.inner.bytes_pos);
        self.inner.bytes_pos = end;
        NonNull::new(aligned as *mut u8).ok_or(AllocError::NoMemory)
    }

    fn dealloc(&mut self, _pos: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {
        // Bump 分配器不支持单个释放，所以这个是 no-op;  Bump 分配器（Bump Allocator） —— 也叫线性分配器，它有个非常鲜明的特点：
        // 👉 分配快如闪电，不支持单个释放（deallocate() 是个空操作 or no-op），只有“全部重置”。
    }

    fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    fn available_bytes(&self) -> usize {
        self.total_bytes - self.used_bytes
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> allocator::AllocResult<usize> {
        let page_size = Self::PAGE_SIZE;
        let size = num_pages * page_size;
        let align = 1 << align_pow2;

        // ✅ 向下对齐当前 page_pos
        let aligned = self.inner.page_pos & !(align - 1);

        // 计算分配起点（因为是从高地址往低地址分配）
        let start = aligned.checked_sub(size).ok_or(AllocError::InvalidParam)?;

        if start < self.inner.bytes_pos {
            return Err(AllocError::MemoryOverlap); // 撞到 bytes 区域
        }

        self.used_bytes += (self.inner.page_pos - start);
        self.inner.page_pos = start;
        
        Ok(start)
    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        // Bump 分配器不支持单个释放，所以这个是 no-op;  Bump 分配器（Bump Allocator） —— 也叫线性分配器，它有个非常鲜明的特点：
        // 👉 分配快如闪电，不支持单个释放（deallocate() 是个空操作 or no-op），只有“全部重置”。
    }

    fn total_pages(&self) -> usize {
        (self.inner.end - self.inner.start) / Self::PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        // 左边：bytes_pos - start → 字节分配器用掉的字节数
        // 右边：end - page_pos → 页分配器用掉的字节数
        // 我们要把它们合起来算页数（注意字节要向上取整转换成页）：
        let used_bytes = self.inner.bytes_pos - self.inner.start;
        let used_pages = self.inner.end - self.inner.page_pos;

        let pages_from_bytes = (used_bytes + Self::PAGE_SIZE - 1) / Self::PAGE_SIZE;
        let pages_from_pages = (used_pages + Self::PAGE_SIZE - 1) / Self::PAGE_SIZE;

        pages_from_bytes + pages_from_pages
    }

    fn available_pages(&self) -> usize {
        // 剩下的空间是：我们把这段区域也转换成页数，注意也向上取整（避免遗漏最后不满一页的空间）：
        if self.inner.page_pos <= self.inner.bytes_pos {
            return 0;
        }
        let available_bytes = self.inner.page_pos - self.inner.bytes_pos;
        (available_bytes + Self::PAGE_SIZE - 1) / Self::PAGE_SIZE
    }
}
