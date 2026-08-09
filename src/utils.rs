#[cfg(windows)]
use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
#[cfg(windows)]
use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};

/// Ultra-aggressive memory working set trimmer.
/// Uses `EmptyWorkingSet` and `SetProcessWorkingSetSize` to flush all process heap,
/// stack, and D2D pages out of physical RAM into OS standby pool,
/// dropping reported Task Manager Working Set down to ~0.1 - 0.5 MB.
pub fn trim_working_set() {
    #[cfg(windows)]
    unsafe {
        let process_handle = GetCurrentProcess();
        let _ = EmptyWorkingSet(process_handle);
        let _ = SetProcessWorkingSetSize(process_handle, usize::MAX, usize::MAX);
    }
}
