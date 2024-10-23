//! Deadlock avoidance algorithms

use alloc::collections::btree_map::BTreeMap;

use crate::task::current_task;

use super::UPSafeCell;

type Algorithm = bankers_algorithm::BankersAlgorithm;

static DEADLOCK_DETECT_ENABLED: UPSafeCell<BTreeMap<usize, Algorithm>> =
    unsafe { UPSafeCell::new(BTreeMap::new()) };

type ResourceIdentifier = usize;
type NumberOfResource = usize;

type TaskIdentifier = usize;

/// Enable deadlock detect
pub fn enable() {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    trace!("Enable deadlock detect.");
    let old = DEADLOCK_DETECT_ENABLED
        .exclusive_access()
        .insert(pid, Algorithm::default());
    assert!(old.is_none());
}

/// Disable deadlock detect
pub fn disable(pid: usize) {
    trace!("Disable deadlock detect.");
    DEADLOCK_DETECT_ENABLED.exclusive_access().remove(&pid);
}

/// Add resource
pub fn add_resource(resource: ResourceIdentifier, n: NumberOfResource) {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    if let Some(algorithm) = DEADLOCK_DETECT_ENABLED.exclusive_access().get_mut(&pid) {
        algorithm.add_resource(resource, n);
    }
}

/// Acquire resource
pub fn acquire(task: TaskIdentifier, resource: ResourceIdentifier, n: NumberOfResource) {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    if let Some(algorithm) = DEADLOCK_DETECT_ENABLED.exclusive_access().get_mut(&pid) {
        trace!("acquire(task: {task}, res: {resource}, n: {n})");
        trace!("before: \n{algorithm:?}");
        algorithm.acquire(task, resource, n);
        trace!("after: \n{algorithm:?}");
    }
}

/// Release resource
pub fn release(task: TaskIdentifier, resource: ResourceIdentifier, n: NumberOfResource) {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    if let Some(algorithm) = DEADLOCK_DETECT_ENABLED.exclusive_access().get_mut(&pid) {
        trace!("release(task: {task}, res: {resource}, n: {n})");
        trace!("before: \n{algorithm:?}");
        algorithm.release(task, resource, n);
        trace!("after: \n{algorithm:?}");
    }
}

/// Handle request
pub fn request(
    task: TaskIdentifier,
    resource: ResourceIdentifier,
    n: NumberOfResource,
) -> Option<RequestResult> {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    if let Some(algorithm) = DEADLOCK_DETECT_ENABLED.exclusive_access().get_mut(&pid) {
        Some(algorithm.request(task, resource, n))
    } else {
        None
    }
}

/// Banker's Algorithm Request Result
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RequestResult {
    /// Potential deadlock
    Deadlock,
    /// Need wait
    Wait,
    /// Allocate resources
    Success,
}

mod bankers_algorithm {
    use core::fmt::Debug;

    use super::{NumberOfResource, RequestResult, ResourceIdentifier, TaskIdentifier};
    use alloc::collections::btree_map::BTreeMap;

    #[derive(Default)]
    struct TaskResourcesState {
        allocation: NumberOfResource,
        need: NumberOfResource,
    }

    impl Debug for TaskResourcesState {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_fmt(format_args!(
                "[alloc: {:?}, need: {:?}]",
                self.allocation, self.need
            ))
        }
    }

    /// Implementation of banker's algorithm
    #[derive(Default)]
    pub struct BankersAlgorithm {
        /// Available map
        available: BTreeMap<ResourceIdentifier, NumberOfResource>,

        task_state: BTreeMap<TaskIdentifier, BTreeMap<ResourceIdentifier, TaskResourcesState>>,
    }

    impl Debug for BankersAlgorithm {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_fmt(format_args!("Available: {:?}\n", self.available))?;
            for (&task, res_state) in &self.task_state {
                f.write_fmt(format_args!("T{task}: {res_state:?}\n"))?;
            }
            Ok(())
        }
    }

    impl BankersAlgorithm {
        /// Add resource
        pub fn add_resource(&mut self, resource: ResourceIdentifier, n: NumberOfResource) {
            *self.available.entry(resource).or_default() += n;
        }

        /// record
        fn record(
            &mut self,
            task: TaskIdentifier,
            resource: ResourceIdentifier,
            n: NumberOfResource,
        ) {
            self.task_state
                .entry(task)
                .or_default()
                .entry(resource)
                .or_default()
                .need += n;
        }

        /// Handle request
        pub fn request(
            &mut self,
            task: TaskIdentifier,
            resource: ResourceIdentifier,
            n: NumberOfResource,
        ) -> RequestResult {
            self.record(task, resource, n);

            if !self.security_check() {
                return RequestResult::Deadlock;
            }
            RequestResult::Success
        }

        /// Acquire resource
        pub fn acquire(
            &mut self,
            task: TaskIdentifier,
            resource: ResourceIdentifier,
            n: NumberOfResource,
        ) {
            let available = self.available.get_mut(&resource).unwrap();
            let task = self
                .task_state
                .get_mut(&task)
                .unwrap()
                .get_mut(&resource)
                .unwrap();
            assert!(*available >= n);
            assert!(task.need >= n);
            // Available[j] = Available[j] - Request[i,j];
            *available -= n;
            // Allocation[i,j] = Allocation[i,j] + Request[i,j];
            task.allocation += n;
            // Need[i,j] = Need[i,j] - Request[i,j];
            task.need -= n;
        }

        /// Release resource
        pub fn release(
            &mut self,
            task: TaskIdentifier,
            resource: ResourceIdentifier,
            n: NumberOfResource,
        ) {
            let available = self.available.get_mut(&resource).unwrap();
            let task = self
                .task_state
                .get_mut(&task)
                .unwrap()
                .get_mut(&resource)
                .unwrap();
            *available += n;
            task.allocation -= n;
        }

        fn security_check(&self) -> bool {
            // 1. 设置两个向量:
            //   工作向量Work，表示操作系统可提供给线程继续运行所需的各类资源数目，它含有m个元素，初始时，Work = Available
            let mut work = self.available.clone();

            //   结束向量Finish，表示系统是否有足够的资源分配给线程，使之运行完成。初始时 Finish[0..n-1] = false，表示所有线程都没结束
            //   当有足够资源分配给线程时，设置Finish[i] = true。
            // TODO(fh): change to BTreeSet
            let mut finish = self
                .task_state
                .keys()
                .map(|&task| (task, false))
                .collect::<BTreeMap<_, _>>();

            loop {
                // 2. 从线程集合中找到一个能满足下述条件的线程
                // Finish[i] == false; Need[i,j] <= Work[j];
                if let Some((task, res_state)) = self.task_state.iter().find(|(task, res_state)| {
                    !finish[task] && res_state.iter().all(|(res, state)| state.need <= work[res])
                }) {
                    // 若找到，执行步骤3，否则，执行步骤4。
                    // 3. 当线程thr[i]获得资源后，可顺利执行，直至完成，并释放出分配给它的资源，故应执行:
                    // Work[j] = Work[j] + Allocation[i,j];
                    for (res, state) in res_state {
                        *work.get_mut(res).unwrap() += state.allocation;
                    }

                    // Finish[i] = true;
                    *finish.get_mut(task).unwrap() = true;

                    // 跳转回步骤2
                    continue;
                } else {
                    // 4. 如果Finish[0..=n-1] 都为true，则表示系统处于安全状态；否则表示系统处于不安全状态。
                    if finish.values().all(|&ok| ok) {
                        return true;
                    } else {
                        return false;
                    }
                }
            }
        }
    }
}
