mod probe;
use std::fs::OpenOptions;
use std::io::Write;
use std::mem::ManuallyDrop;

use windows::core::{Interface, IUnknown, HSTRING};
use windows::ApplicationModel::Core::CoreApplication;
use windows::UI::Core::CoreWindow;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};

unsafe fn read_wide(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}

fn log(dir: &str, msg: &str) {
    let path = format!("{}\\horizon_xbox.log", dir);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{}", msg);
    }
}

fn get_core_window() -> windows::core::Result<CoreWindow> {
    let props = CoreApplication::Properties()?;
    let obj = props.Lookup(&HSTRING::from("EGLNativeWindowTypeProperty"))?;
    obj.cast::<CoreWindow>()
}

fn transition(
    res: &ID3D12Resource,
    before: D3D12_RESOURCE_STATES,
    after: D3D12_RESOURCE_STATES,
) -> D3D12_RESOURCE_BARRIER {
    D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: ManuallyDrop::new(Some(res.clone())),
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                StateBefore: before,
                StateAfter: after,
            }),
        },
    }
}

fn render_test(dir: &str, window: &CoreWindow, width: u32, height: u32) -> windows::core::Result<()> {
    unsafe {
        log(dir, "step: create DXGI factory");
        let factory: IDXGIFactory2 = CreateDXGIFactory1()?;

        log(dir, "step: create D3D12 device");
        let mut device: Option<ID3D12Device> = None;
        D3D12CreateDevice(None::<&IUnknown>, D3D_FEATURE_LEVEL_11_0, &mut device)?;
        let device = device.ok_or_else(windows::core::Error::empty)?;

        log(dir, "step: create command queue");
        let qdesc = D3D12_COMMAND_QUEUE_DESC {
            Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
            ..Default::default()
        };
        let queue: ID3D12CommandQueue = device.CreateCommandQueue(&qdesc)?;

        log(dir, &format!("step: create swapchain on CoreWindow {}x{}", width, height));
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width,
            Height: height,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            Stereo: false.into(),
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_NONE,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE,
            Flags: 0,
        };
        let swap: IDXGISwapChain1 =
            factory.CreateSwapChainForCoreWindow(&queue, window, &desc, None::<&IDXGIOutput>)?;
        let swap3: IDXGISwapChain3 = swap.cast()?;

        log(dir, "step: create render target views");
        let heap: ID3D12DescriptorHeap = device.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            NumDescriptors: 2,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
            NodeMask: 0,
        })?;
        let inc = device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) as usize;
        let start = heap.GetCPUDescriptorHandleForHeapStart();
        let mut targets: Vec<(ID3D12Resource, D3D12_CPU_DESCRIPTOR_HANDLE)> = Vec::new();
        for i in 0..2u32 {
            let buf: ID3D12Resource = swap.GetBuffer(i)?;
            let h = D3D12_CPU_DESCRIPTOR_HANDLE { ptr: start.ptr + (i as usize) * inc };
            device.CreateRenderTargetView(&buf, None, h);
            targets.push((buf, h));
        }

        let alloc: ID3D12CommandAllocator =
            device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)?;
        let list: ID3D12GraphicsCommandList =
            device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &alloc, None::<&ID3D12PipelineState>)?;
        list.Close()?;
        let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE)?;
        let event = CreateEventW(None, false, false, None)?;
        let mut fence_val: u64 = 0;

        log(dir, "step: drawing 300 frames");
        for frame in 0..300u32 {
            let idx = swap3.GetCurrentBackBufferIndex() as usize;
            let (buf, rtv) = &targets[idx];
            alloc.Reset()?;
            list.Reset(&alloc, None::<&ID3D12PipelineState>)?;
            list.ResourceBarrier(&[transition(
                buf,
                D3D12_RESOURCE_STATE_PRESENT,
                D3D12_RESOURCE_STATE_RENDER_TARGET,
            )]);
            let t = frame as f32 / 300.0;
            let color = [t, 0.2, 1.0 - t, 1.0];
            list.ClearRenderTargetView(*rtv, &color, None);
            list.ResourceBarrier(&[transition(
                buf,
                D3D12_RESOURCE_STATE_RENDER_TARGET,
                D3D12_RESOURCE_STATE_PRESENT,
            )]);
            list.Close()?;
            let cl: ID3D12CommandList = list.cast()?;
            queue.ExecuteCommandLists(&[Some(cl)]);
            swap.Present(1, DXGI_PRESENT(0)).ok()?;
            fence_val += 1;
            queue.Signal(&fence, fence_val)?;
            if fence.GetCompletedValue() < fence_val {
                fence.SetEventOnCompletion(fence_val, event)?;
                WaitForSingleObject(event, INFINITE);
            }
        }
        let _ = CloseHandle(event);
        log(dir, "step: done, releasing graphics objects");
        drop(targets);
        drop(swap3);
        drop(swap);
        Ok(())
    }
}

#[no_mangle]
pub unsafe extern "system" fn horizon_xbox_run(local_state: *const u16) -> i32 {
    let dir = read_wide(local_state);
    log(&dir, "horizon_xbox_run called");
    let window = match get_core_window() {
        Ok(w) => w,
        Err(e) => {
            log(&dir, &format!("no CoreWindow: {e}"));
            return 1;
        }
    };
    let b = match window.Bounds() {
        Ok(b) => b,
        Err(e) => {
            log(&dir, &format!("Bounds() failed: {e}"));
            return 2;
        }
    };
    log(&dir, &format!("bounds: {}x{}", b.Width, b.Height));
    #[cfg(horizon_linked)]
    {
        let f: fn() = horizon_client::entry::main;
        log(&dir, &format!("horizon linked, entry at {:p}", f as *const ()));
    }
    probe::probe_dlls(&dir);
    probe::probe_imports(&dir);
    probe::try_load_engine(&dir);
    return 0;
    match render_test(&dir, &window, b.Width as u32, b.Height as u32) {
        Ok(()) => {
            log(&dir, "render test OK");
            0
        }
        Err(e) => {
            log(&dir, &format!("render test FAILED: {e}"));
            3
        }
    }
}
