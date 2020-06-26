use core::fmt::Debug;
use core::ops::Range;

use crate::{Arch, Tid};

/// A collection of methods and metadata GDB can use to control + query a
/// running system.
///
/// This trait describes the architecture and capabilities of a target system,
/// and provides an interface for `GdbStub` to modify and control the system's
/// state.
///
/// There are several [provided methods](#provided-methods) that can optionally
/// be implemented to enable additional advanced GDB debugging functionality.
///
/// ### What's with the `<Self::Arch as Arch>::` syntax?
///
/// Yeah, sorry about that!
///
/// If [rust-lang/rust#38078](https://github.com/rust-lang/rust/issues/38078)
/// every gets fixed, `<Self::Arch as Arch>::Usize` can be simplified to the
/// much more readable `Self::Arch::Usize`.
///
/// Until then, when implementing `Target`, I recommend using the concrete
/// type directly. (e.g: on a 32-bit platform, instead of writing `<Self::Arch
/// as Arch>::Usize`, just use `u32` directly)
pub trait Target {
    /// The target's architecture.
    type Arch: Arch;

    /// A target-specific fatal error.
    type Error;

    /// Resume execution, specifying different resume actions for each thread.
    ///
    /// _Author's recommendation:_ If you're implementing `Target` to debug
    /// bare-metal code (emulated or not), treat the `tid` field as a _core_ ID
    /// (as threads are an OS-level construct).
    fn resume(
        &mut self,
        actions: impl Iterator<Item = (Tid, ResumeAction)>,
    ) -> Result<StopReason<<Self::Arch as Arch>::Usize>, Self::Error>;

    /// Read the target's registers.
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as Arch>::Registers,
    ) -> Result<(), Self::Error>;

    /// Write the target's registers.
    fn write_registers(
        &mut self,
        regs: &<Self::Arch as Arch>::Registers,
    ) -> Result<(), Self::Error>;

    /// Read bytes from the specified address range.
    fn read_addrs(
        &mut self,
        addrs: Range<<Self::Arch as Arch>::Usize>,
        val: impl FnMut(u8),
    ) -> Result<(), Self::Error>;

    /// Write bytes to the specified address range.
    fn write_addrs(
        &mut self,
        get_addr_val: impl FnMut() -> Option<(<Self::Arch as Arch>::Usize, u8)>,
    ) -> Result<(), Self::Error>;

    /// Set/remove a software breakpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    ///
    /// See [this stackoverflow](https://stackoverflow.com/questions/8878716/what-is-the-difference-between-hardware-and-software-breakpoints)
    /// discussion about the differences between hardware and software
    /// breakpoints.
    ///
    /// _Author's recommendation:_ If you're implementing `Target` for an
    /// emulator using an _interpreted_ CPU (as opposed to a JIT), the
    /// simplest way to implement "software" breakpoints is to check the
    /// `PC` value after each CPU cycle.
    fn update_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        op: BreakOp,
    ) -> Result<bool, Self::Error>;

    /// (optional) Set/remove a hardware breakpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    ///
    /// See [this stackoverflow](https://stackoverflow.com/questions/8878716/what-is-the-difference-between-hardware-and-software-breakpoints)
    /// discussion about the differences between hardware and software
    /// breakpoints.
    ///
    /// _Author's recommendation:_ If you're implementing `Target` for an
    /// emulator using an _interpreted_ CPU (as opposed to a JIT), there
    /// shouldn't be any reason to implement this method (as software
    /// breakpoints are likely to be just-as-fast).
    fn update_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        op: BreakOp,
    ) -> Option<Result<bool, Self::Error>> {
        let _ = (addr, op);
        None
    }

    /// (optional) Set/remove a hardware watchpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    ///
    /// See the [GDB documentation](https://sourceware.org/gdb/current/onlinedocs/gdb/Set-Watchpoints.html)
    /// regarding watchpoints.
    ///
    /// _NOTE:_ If this method isn't implemented, GDB will default to using
    /// _software watchpoints_, which tend to be excruciatingly slow (as
    /// they are implemented by single-stepping the system, and reading the
    /// memory location after each step).
    fn update_hw_watchpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        op: BreakOp,
        kind: WatchKind,
    ) -> Option<Result<bool, Self::Error>> {
        let _ = (addr, op, kind);
        None
    }
}

/// What kind of watchpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchKind {
    /// Fire when the memory location is written to.
    Write,
    /// Fire when the memory location is read from.
    Read,
    /// Fire when the memory location is written to and/or read from.
    ReadWrite,
}

/// Add / Remove a breakpoint / watchpoint
#[derive(Debug)]
pub enum BreakOp {
    /// Add a new breakpoint / watchpoint.
    Add,
    /// Remove an existing breakpoint / watchpoint.
    Remove,
}

/// The system's current execution state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason<U> {
    /// Running
    Running,
    /// Halted
    Halted,
    /// Hit a software breakpoint (e.g. due to a trap instruction).
    ///
    /// NOTE: This does not necessarily have to be a breakpoint configured by
    /// the client/user of the current GDB session.
    SwBreak,
    /// Hit a hardware breakpoint.
    HwBreak,
    /// Hit a watchpoint.
    Watch {
        /// Kind of watchpoint that was hit
        kind: WatchKind,
        /// Address of watched memory
        addr: U,
    },
}

/// Describes how the target should resume the specified thread.
pub enum ResumeAction {
    /// Continue execution (until the next event occurs).
    Continue,
    /// Step forward a single instruction.
    Step,
    /* ContinueWithSignal(u8),
     * StepWithSignal(u8),
     * Stop,
     * StepInRange(core::ops::Range<U>), */
}