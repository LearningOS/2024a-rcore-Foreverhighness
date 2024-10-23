# 吐槽时间

## Reduce Construct of Arc

Arc 如果不需要所有权的话完全是可以传引用的。

```rust
impl TaskUserRes {
    /// Create a new TaskUserRes (Task User Resource)
    pub fn new(
        process: &Arc<ProcessControlBlock>,
        ustack_base: usize,
        alloc_user_res: bool,
    ) -> Self {
        let tid = process.inner_exclusive_access().alloc_tid();
        let task_user_res = Self {
            tid,
            ustack_base,
            process: Arc::downgrade(process),
        };
        if alloc_user_res {
            task_user_res.alloc_user_res();
        }
        task_user_res
    }
}
```

## non-canonical implementation of `partial_cmp` on an `Ord` type 

`TimeCondVar` 的 `Ord` 实现的不是很地道。  
实际上在 `BinaryHeap` 的官方文档里就有[`最小堆`](https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html#min-heap)的推荐写法。  
如果开启了 `clippy` 的话也会报 [`non_canonical_partial_ord_impl`](https://rust-lang.github.io/rust-clippy/master/index.html#non_canonical_partial_ord_impl) lint.

```rust
impl PartialOrd for TimerCondVar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TimerCondVar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.expire_ms.cmp(&other.expire_ms).reverse()
    }
}
```

