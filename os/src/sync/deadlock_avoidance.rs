//! Deadlock avoidance algorithms

use super::UPSafeCell;

type Algorithm = bankers_algorithm::BankersAlgorithm;

static DEADLOCK_DETECT_ENABLED: UPSafeCell<Option<Algorithm>> = unsafe { UPSafeCell::new(None) };

type ResourceIdentifier = usize;
type NumberOfResource = usize;

type TaskIdentifier = usize;

/// Enable deadlock detect
pub fn enable() {
    debug!("Enable deadlock detect enabled");
    *DEADLOCK_DETECT_ENABLED.exclusive_access() = Some(Algorithm::default());
}

/// Disable deadlock detect
pub fn disable() {
    debug!("Disable deadlock detect.");
    DEADLOCK_DETECT_ENABLED.exclusive_access().take();
}

/// Add resource
pub fn add_resource(resource: ResourceIdentifier, n: NumberOfResource) {
    if let Some(algorithm) = DEADLOCK_DETECT_ENABLED.exclusive_access().as_mut() {
        algorithm.add_resource(resource, n);
    }
}

/// Handle request
pub fn acquire(
    task: TaskIdentifier,
    resource: ResourceIdentifier,
    n: NumberOfResource,
) -> Option<RequestResult> {
    let mut algorithm = DEADLOCK_DETECT_ENABLED.exclusive_access();
    let algorithm = algorithm.as_mut()?;
    Some(algorithm.acquire(task, resource, n))
}

/// Release resource
pub fn release(task: TaskIdentifier, resource: ResourceIdentifier, n: NumberOfResource) {
    if let Some(algorithm) = DEADLOCK_DETECT_ENABLED.exclusive_access().as_mut() {
        algorithm.release(task, resource, n);
    }
}

/// Banker's Algorithm Request Result
#[derive(Debug, PartialEq)]
pub enum RequestResult {
    /// Potential deadlock
    Deadlock,
    /// Need wait
    Wait,
    /// Allocate resources
    Success,
}

mod bankers_algorithm {
    use super::{NumberOfResource, RequestResult, ResourceIdentifier, TaskIdentifier};
    use alloc::collections::btree_map::BTreeMap;

    #[derive(Debug, Default)]
    struct TaskResourcesState {
        // max: NumberOfResource,
        allocation: NumberOfResource,
        need: NumberOfResource,
    }

    /// Implementation of banker's algorithm
    #[derive(Debug, Default)]
    pub struct BankersAlgorithm {
        /// Available map
        available: BTreeMap<ResourceIdentifier, NumberOfResource>,

        task_state: BTreeMap<TaskIdentifier, BTreeMap<ResourceIdentifier, TaskResourcesState>>,
    }

    fn default_res_state_from_available(
        available: &BTreeMap<ResourceIdentifier, NumberOfResource>,
    ) -> BTreeMap<ResourceIdentifier, TaskResourcesState> {
        available
            .iter()
            .map(|(&res, &need)| {
                (
                    res,
                    TaskResourcesState {
                        allocation: 0,
                        need,
                    },
                )
            })
            .collect()
    }

    impl BankersAlgorithm {
        /// Add resource
        pub fn add_resource(&mut self, resource: ResourceIdentifier, n: NumberOfResource) {
            *self.available.entry(resource).or_default() += n;
        }

        /// Handle request
        pub fn acquire(
            &mut self,
            task: TaskIdentifier,
            resource: ResourceIdentifier,
            n: NumberOfResource,
        ) -> RequestResult {
            let task_state = &self
                .task_state
                .entry(task)
                .or_insert_with(|| default_res_state_from_available(&self.available))[&resource];
            let available = self.available[&resource];

            // 1. 如果 Request[i,j] ≤ Need[i,j]，则转步骤2；否则出错，因为线程所需的资源数已超过它所宣布的最大值。
            if n > task_state.need {
                return RequestResult::Deadlock;
            }

            // 2. 如果 Request[i,j] ≤ Available[j]，则转步骤3；否则，表示尚无足够资源，线程thr[i]进入等待状态。
            if n > available {
                return RequestResult::Wait;
            }

            // 3. 操作系统试着把资源分配给线程thr[i]，并修改下面数据结构中的值：
            self.alloc(task, resource, n);

            // 4. 操作系统执行安全性检查算法，检查此次资源分配后系统是否处于安全状态。若安全，则实际将资源分配给线程thr[i]；否则不进行资源分配，让线程thr[i]等待。
            if !self.security_check() {
                self.dealloc(task, resource, n);
                return RequestResult::Wait;
            }

            RequestResult::Success
        }

        /// Release resource
        pub fn release(
            &mut self,
            task: TaskIdentifier,
            resource: ResourceIdentifier,
            n: NumberOfResource,
        ) {
            self.dealloc(task, resource, n);
        }

        fn alloc(
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
            // Available[j] = Available[j] - Request[i,j];
            *available -= n;
            // Allocation[i,j] = Allocation[i,j] + Request[i,j];
            task.allocation += n;
            // Need[i,j] = Need[i,j] - Request[i,j];
            task.need -= n;
        }

        fn dealloc(
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
            task.need += n;
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
                // let i = for (task, res_state) in &self.task_state {
                //     for (res, state) in res_state {
                //         if !finish[task] && state.need <= work[res] {
                //             break Some((*task, *res, state.allocation));
                //         }
                //     }
                // };
                if let Some((task, res, allocated)) =
                    self.task_state.iter().find_map(|(task, res_state)| {
                        (!finish[task])
                            .then(|| {
                                res_state
                                    .iter()
                                    .find(|&(res, state)| state.need <= work[res])
                                    .map(|(res, state)| (task, res, state.allocation))
                            })
                            .flatten()
                    })
                {
                    // 若找到，执行步骤3，否则，执行步骤4。
                    // 3. 当线程thr[i]获得资源后，可顺利执行，直至完成，并释放出分配给它的资源，故应执行:
                    // Work[j] = Work[j] + Allocation[i,j];
                    *work.get_mut(res).unwrap() += allocated;

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
