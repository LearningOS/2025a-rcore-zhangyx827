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
在`ready_queue`, 与 `TIMERS` 中的线程需要被回收，因为它们两个不是进程级别的，如果不回收会导致内存泄漏。 
在锁和信号量的`wait_queue`中的不需要手动回收，因为这属于进程级别的，在`ProcessControlBlock`被回收之后，自然会被回收
对比以下两种 Mutex 中的实现，二者有什么区别？这些区别可能会导致什么问题？

impl Mutex for Mutex1 {
    fn lock(&self) {
        loop {
            let mut mutex_inner = self.inner.exclusive_access();
            if mutex_inner.locked {
                mutex_inner.wait_queue.push_back(current_task().unwrap());
                drop(mutex_inner);
                block_current_and_run_next();
            } else {
                mutex_inner.locked = true;
                break;
            }
        }
    }

    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        mutex_inner.locked = false;
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        }
    }
}

impl Mutex for Mutex2 {
    fn lock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
    }

    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}

区别在于第二种实现先判断`wait_queue`是否为空，如果不为空直接转移锁的所有者

## 荣誉准则

在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
无

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。