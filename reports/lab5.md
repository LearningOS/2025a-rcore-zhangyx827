## lab5实验报告

|系统调用| 功能实现|
| --|--|
|`sys_enable_deadlock_detect`|在`ProcessControlBlock`中，我加入了`enable_detct`字段，在系统调用中，将此字段设置为   true`表示对该进程启用死锁检测|
---
在`TaskControlBlock`中，我加入了`res_holding`与`res_earning`字段，表示拥有和渴望得到的锁或者信号量，当线程得不到资源，就设置自己的`res_earning`，当线程的消耗了资源，就将资源放入到`res_holding`当中，`值得注意的是`，对于会发生block的信号量或者锁的实现，在拥有该资源的线程释放了资源之后，阻塞的线程唤醒之前，有一段资源没有被任何线程利用的`时间段`，为了能够将资源视为可用，就需要在锁当中增加新的字段来标识。`sync::mod.rs`中，我实现了`deadlock_detect_ok`的`helper_function`。在函数中，先根据当前检测的是`mutex`还是`semaphore`将还没被利用的锁或者信号量加入到`available_res`中。将能够因为在`available_res`中找到`earning_res`的任务称为`可完成任务`，通过循环来检测任务是否能够称为可完成任务。最后如果存在不能完成的任务就说明检测到死锁。

完成时间：两天

## 问答作业

在我们的多线程实现中，当主线程 (即 0 号线程) 退出时，视为整个进程退出， 此时需要结束该进程管理的所有线程并回收其资源。 - 需要回收的资源有哪些？ - 其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？
- 资源包括tid, trap_cx, 线程的用户栈和内核栈，trap上下文的页面所占用的物理空间.
- 可能在`PROCESSOR`的`ready_queue`,  `TIMERS`, 以及锁和信号量的`wait_queue`中。
在`ready_queue`, 与 `TIMERS` 中的线程需要被回收，因为
对比以下两种 Mutex 中的实现，二者有什么区别？这些区别可能会导致什么问题？
 1impl Mutex for Mutex1 {
 2    fn lock(&self) {
 3        loop {
 4            let mut mutex_inner = self.inner.exclusive_access();
 5            if mutex_inner.locked {
 6                mutex_inner.wait_queue.push_back(current_task().unwrap());
 7                drop(mutex_inner);
 8                block_current_and_run_next();
 9            } else {
10                mutex_inner.locked = true;
11                break;
12            }
13        }
14    }
15
16    fn unlock(&self) {
17        let mut mutex_inner = self.inner.exclusive_access();
18        assert!(mutex_inner.locked);
19        mutex_inner.locked = false;
20        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
21            add_task(waking_task);
22        }
23    }
24}
25
26impl Mutex for Mutex2 {
27    fn lock(&self) {
28        let mut mutex_inner = self.inner.exclusive_access();
29        if mutex_inner.locked {
30            mutex_inner.wait_queue.push_back(current_task().unwrap());
31            drop(mutex_inner);
32            block_current_and_run_next();
33        } else {
34            mutex_inner.locked = true;
35        }
36    }
37
38    fn unlock(&self) {
39        let mut mutex_inner = self.inner.exclusive_access();
40        assert!(mutex_inner.locked);
41        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
42            add_task(waking_task);
43        } else {
44            mutex_inner.locked = false;
45        }
46    }
47}
