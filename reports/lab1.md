## lab1实验报告

### 实现的功能

在`TaskControlBlock`中，我新增加了一个`CountSyscall`类，用来计数`每一个任务`系统调用的次数。

|函数 | 功能|
| --- | ---  |
|`get_cnt` | 通过`match`系统调用号来得到`当前任务`的对应的系统调用次数|
| `modify_cnt` | 通过`match`系统调用号，增加`当前任务`的对应的系统调用的次数

### 简答作业
1. 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。
- 运行`ch2_bad_address.rs`  输出为
```
unsafe precondition(s) violated: ptr::write_volatile requires that the pointer argument is aligned and non-null
```
- `ch2_bad_instructions.rs`
```
[kernel] IllegalInstruction in application, kernel killed it.
```
- `ch2_bad_resgister.rs`
```
[kernel] IllegalInstruction in application, kernel killed it.
```
- SBI版本：RustSBI version 0.3.0-alpha.2


2. 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:
L40：刚进入 `__restore` 时，`sp` 代表了什么值。请指出 __restore 的两种使用情景。
答： 
- 刚进入`__restore`时，`sp`代表用户栈栈顶的地址
- `__restore`用处
    - 调用`trap_handler`之后 内核栈栈顶Trap上下文恢复通用寄存器和CSR 并且回收内核栈上面的Trap上下文所占用的内存 回归进入内核之前的内核栈的栈顶 
    - 启动应用程序 通过在内核栈上压入一个Trap上下文 `sepc`是应用程序的入口地址 `sp`指向用户栈，从而在`__restore`中启动应用程序
2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释
```
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2
```
- 特殊处理了`sstatus`,`sepc`,`sscratch`寄存器，`sepc`寄存器的设置能够使程序计数器在返回用户态的时候被设置为正确的值，确保用户程序从被打断的位置继续进行，`sscratch`设置为用户栈栈顶地址，方便后续将其值与`sp`寄存器切换，使得`sp`保存用户栈栈顶地址，确保了用户程序能够使用自己的栈空间。`sret`指令执行过程中，借助`sstatus`寄存器中的`SPP`字段值，CPU可以正确将特权级设置为用户态.

3. L50-L56：为何跳过了 `x2` 和 `x4`？
- `x2` 指向`内核栈`,`用户栈`保存在`sscratch`之中，因此先考虑保留其他寄存器
- `x4` 寄存器，除非出于一些特殊用途使用它，否则一般不会被用到

4. L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？
```
csrrw sp, sscratch, sp
```

- `sp`中的值会是用户栈栈顶地址 `sscratch`中的值会是内核栈栈顶地址

5. __restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

- `sret`指令执行阶段CPU会将当前的特权级按照`sstatus`中的`SPP`字段设置为U或者S

6. L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？
csrrw sp, sscratch, sp

- `sp`中的值会是内核栈栈顶地址，`sscratch`中的值会是用户栈栈顶地址

7. 从 U 态进入 S 态是哪一条指令发生的？
- ecall





### 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：


2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

`task::context.rs`中的`zero_init`的命名

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计


