$ErrorActionPreference = "Stop"
$dir = "horizon-src\third_party\wgpu-hal\src\dx12"

function Edit-File([string]$name, [object[]]$pairs) {
    $path = (Resolve-Path (Join-Path $dir $name)).Path
    $t = [System.IO.File]::ReadAllText($path).Replace("`r`n", "`n")
    foreach ($p in $pairs) {
        $count = ([regex]::Matches($t, [regex]::Escape($p[0]))).Count
        if ($count -ne 1) { throw "${name}: expected exactly one match for [$($p[0])] but found $count" }
        $t = $t.Replace($p[0], $p[1])
    }
    [System.IO.File]::WriteAllText($path, $t, (New-Object System.Text.UTF8Encoding($false)))
}

$ctor = @'
    pub unsafe fn create_surface_from_core_window(&self, core_window: *mut ffi::c_void) -> Surface {
        let core_window = unsafe { windows::core::IUnknown::from_raw_borrowed(&core_window) }
            .expect("COM pointer should not be NULL");
        Surface {
            factory: self.factory.clone(),
            factory_media: self.factory_media.clone(),
            target: SurfaceTarget::CoreWindow(core_window.to_owned()),
            supports_allow_tearing: self.supports_allow_tearing,
            swap_chain: RwLock::new(None),
            options: self.options.clone(),
            hdr_source: None,
        }
    }

'@

$swapArm = @'
SurfaceTarget::CoreWindow(ref window) => {
                        profiling::scope!("IDXGIFactory2::CreateSwapChainForCoreWindow");
                        unsafe {
                            self.factory.CreateSwapChainForCoreWindow(
                                &device.present_queue,
                                window,
                                &desc,
                                None::<&Dxgi::IDXGIOutput>,
                            )
                        }
                    }
                    SurfaceTarget::SurfaceHandle(handle) => {
'@

Edit-File "mod.rs" @(
    @('SwapChainPanel(types::ISwapChainPanelNative),', "SwapChainPanel(types::ISwapChainPanelNative),`n    /// UWP / Xbox CoreWindow, held as IUnknown.`n    CoreWindow(windows::core::IUnknown),"),
    @('    pub unsafe fn create_surface_from_swap_chain_panel(', ($ctor + '    pub unsafe fn create_surface_from_swap_chain_panel(')),
    @('SurfaceTarget::SurfaceHandle(handle) => {', $swapArm),
    @('SurfaceTarget::SwapChainPanel(swap_chain_panel) => {', "SurfaceTarget::CoreWindow(_) => {}`n                    SurfaceTarget::SwapChainPanel(swap_chain_panel) => {"),
    @('| SurfaceTarget::SwapChainPanel(_) => {}', "| SurfaceTarget::SwapChainPanel(_)`n            | SurfaceTarget::CoreWindow(_) => {}")
)

Edit-File "adapter.rs" @(
    @('| SurfaceTarget::SwapChainPanel(_) => None,', "| SurfaceTarget::SwapChainPanel(_)`n                | SurfaceTarget::CoreWindow(_) => None,"),
    @('SurfaceTarget::WndHandle(_) => vec![wgt::CompositeAlphaMode::Opaque],', "SurfaceTarget::WndHandle(_) | SurfaceTarget::CoreWindow(_) => {`n                    vec![wgt::CompositeAlphaMode::Opaque]`n                }")
)

$oldArm = @'
            _ => Err(crate::InstanceError::new(format!(
                "window handle {window_handle:?} is not a Win32 handle"
'@
$newArm = @'
            raw_window_handle::RawWindowHandle::WinRt(handle) => {
                Ok(unsafe { self.create_surface_from_core_window(handle.core_window.as_ptr()) })
            }
            _ => Err(crate::InstanceError::new(format!(
                "window handle {window_handle:?} is not a Win32 handle"
'@
Edit-File "instance.rs" @( ,@($oldArm, $newArm) )

Write-Host "wgpu-hal patched for CoreWindow"
